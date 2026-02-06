use crate::types::net_game_packet::{NetGamePacket, NetGamePacketData};
use crate::types::net_message::NetMessage;
use crate::types::status::PeerStatus;
use crate::Bot;
use mlua::{Lua, UserData, UserDataFields, UserDataMethods};
use std::collections::HashMap;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};

// ── Core bot types ──────────────────────────────────────────────

#[derive(Debug, Default)]
pub struct State {
    pub hack_type: u32,
    pub build_length: u8,
    pub punch_length: u8,
    pub velocity: f32,
    pub gravity: f32,
}

#[derive(Debug, Clone, Copy)]
pub struct DelayConfig {
    pub findpath_delay: u32,
    pub punch_delay: u32,
    pub place_delay: u32,
}

impl Default for DelayConfig {
    fn default() -> Self {
        Self {
            findpath_delay: 150,
            punch_delay: 100,
            place_delay: 100,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Automation {
    pub auto_collect: bool,
    pub auto_reconnect: bool,
}

impl Default for Automation {
    fn default() -> Self {
        Self {
            auto_collect: true,
            auto_reconnect: true,
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum LoginVia {
    GOOGLE,
    APPLE,
    LTOKEN([String; 4]),
    LEGACY([String; 2]),
}

impl Default for LoginVia {
    fn default() -> Self {
        LoginVia::LEGACY([String::new(), String::new()])
    }
}

#[derive(Default)]
pub struct TemporaryData {
    pub drop: Mutex<(u32, u32)>,
    pub trash: Mutex<(u32, u32)>,
    pub dialog_callback: Mutex<Option<fn(&Bot)>>,
}

// ── Scripting & Callback System ─────────────────────────────────

pub struct LuaCallback {
    pub key: mlua::RegistryKey,
    pub once: bool,
}

pub struct Scripting {
    pub data: Mutex<String>,
    pub currently_executing: AtomicBool,
    pub lua: Lua,
    pub callbacks: Mutex<HashMap<String, Vec<LuaCallback>>>,
}

impl Default for Scripting {
    fn default() -> Self {
        Scripting {
            data: Mutex::new(String::new()),
            currently_executing: AtomicBool::new(false),
            lua: Lua::new(),
            callbacks: Mutex::new(HashMap::new()),
        }
    }
}

// ── Lua UserData: BotArc ────────────────────────────────────────

#[derive(Clone)]
pub struct BotArc(pub Arc<Bot>);

impl UserData for BotArc {
    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        // ── Actions ──
        methods.add_method("say", |_, this, message: String| {
            this.0.say(&message);
            Ok(())
        });
        methods.add_method("warp", |_, this, world_name: String| {
            this.0.warp(world_name);
            Ok(())
        });
        methods.add_method("leave", |_, this, ()| {
            this.0.leave();
            Ok(())
        });
        methods.add_method("disconnect", |_, this, ()| {
            this.0.disconnect();
            Ok(())
        });
        methods.add_method("punch", |_, this, (ox, oy): (i32, i32)| {
            this.0.punch(ox, oy);
            Ok(())
        });
        methods.add_method("place", |_, this, (ox, oy, id): (i32, i32, u32)| {
            this.0.place(ox, oy, id, false);
            Ok(())
        });
        methods.add_method("wrench", |_, this, (ox, oy): (i32, i32)| {
            this.0.wrench(ox, oy);
            Ok(())
        });
        methods.add_method("wrenchPlayer", |_, this, net_id: u32| {
            this.0.wrench_player(net_id);
            Ok(())
        });
        methods.add_method("wear", |_, this, item_id: u32| {
            this.0.wear(item_id);
            Ok(())
        });
        methods.add_method("drop", |_, this, (id, amount): (u32, u32)| {
            this.0.drop_item(id, amount);
            Ok(())
        });
        methods.add_method("trash", |_, this, (id, amount): (u32, u32)| {
            this.0.trash_item(id, amount);
            Ok(())
        });
        methods.add_method("collect", |_, this, ()| Ok(this.0.collect()));
        methods.add_method("acceptAccess", |_, this, ()| {
            this.0.accept_access();
            Ok(())
        });
        methods.add_method("hasAccess", |_, this, ()| Ok(this.0.has_access()));
        methods.add_method("enterDoor", |_, this, (ox, oy): (i32, i32)| {
            this.0.enter_door(ox, oy);
            Ok(())
        });
        methods.add_method("sendDialogReturn", |_, this, data: String| {
            this.0.send_dialog_return(&data);
            Ok(())
        });

        // ── Movement ──
        methods.add_method("walk", |_, this, (ox, oy): (i32, i32)| {
            this.0.walk(ox, oy, false);
            Ok(())
        });
        methods.add_method("findPath", |_, this, (x, y): (u32, u32)| {
            this.0.find_path(x, y);
            Ok(())
        });

        // ── Config ──
        methods.add_method("setAutoCollect", |_, this, on: bool| {
            this.0.set_auto_collect(on);
            Ok(())
        });
        methods.add_method("setAutoReconnect", |_, this, on: bool| {
            this.0.set_auto_reconnect(on);
            Ok(())
        });
        methods.add_method("setFindPathDelay", |_, this, ms: u32| {
            this.0.set_findpath_delay(ms);
            Ok(())
        });
        methods.add_method("setPunchDelay", |_, this, ms: u32| {
            this.0.set_punch_delay(ms);
            Ok(())
        });
        methods.add_method("setPlaceDelay", |_, this, ms: u32| {
            this.0.set_place_delay(ms);
            Ok(())
        });

        // ── Raw Packets ──
        methods.add_method(
            "sendTextPacket",
            |_, this, (msg_type, text): (u32, String)| {
                this.0
                    .send_text_packet(NetMessage::from(msg_type), text.as_bytes());
                Ok(())
            },
        );
        methods.add_method("sendGamePacket", |_, this, pkt: LuaGamePacket| {
            this.0.send_game_packet(&pkt.0, None, true);
            Ok(())
        });
        methods.add_method(
            "sendGamePacketRaw",
            |_, this, (pkt, reliable): (LuaGamePacket, bool)| {
                this.0.send_game_packet(&pkt.0, None, reliable);
                Ok(())
            },
        );

        // ── Event System ──
        methods.add_method("on", |lua, this, (event, func): (String, mlua::Function)| {
            let key = lua.create_registry_value(func)?;
            let mut cbs = this.0.scripting.callbacks.lock().unwrap();
            cbs.entry(event).or_default().push(LuaCallback { key, once: false });
            Ok(())
        });
        methods.add_method("once", |lua, this, (event, func): (String, mlua::Function)| {
            let key = lua.create_registry_value(func)?;
            let mut cbs = this.0.scripting.callbacks.lock().unwrap();
            cbs.entry(event).or_default().push(LuaCallback { key, once: true });
            Ok(())
        });
        methods.add_method("removeListener", |lua, this, event: String| {
            let mut cbs = this.0.scripting.callbacks.lock().unwrap();
            if let Some(callbacks) = cbs.remove(&event) {
                for cb in callbacks {
                    lua.remove_registry_value(cb.key)?;
                }
            }
            Ok(())
        });
        methods.add_method("removeAllListeners", |lua, this, ()| {
            let mut cbs = this.0.scripting.callbacks.lock().unwrap();
            for (_, callbacks) in cbs.drain() {
                for cb in callbacks {
                    lua.remove_registry_value(cb.key)?;
                }
            }
            Ok(())
        });
    }

    fn add_fields<F: UserDataFields<Self>>(fields: &mut F) {
        fields.add_field_method_get("pos", |_, this| {
            let pos = this.0.movement.position();
            Ok(LuaPosition(pos.0, pos.1))
        });
        fields.add_field_method_get("tile", |lua, this| {
            let pos = this.0.movement.position();
            let t = lua.create_table()?;
            t.set("x", (pos.0 / 32.0).floor() as i32)?;
            t.set("y", (pos.1 / 32.0).floor() as i32)?;
            Ok(t)
        });
        fields.add_field_method_get("gems", |_, this| Ok(this.0.inventory.gems()));
        fields.add_field_method_get("netId", |_, this| Ok(this.0.runtime.net_id()));
        fields.add_field_method_get("userId", |_, this| Ok(this.0.runtime.user_id()));
        fields.add_field_method_get("name", |_, this| {
            let info = this.0.auth.login_info();
            Ok(info.as_ref().map(|i| i.tank_id_name.clone()).unwrap_or_default())
        });
        fields.add_field_method_get("world", |_, this| Ok(LuaWorld(this.0.clone())));
        fields.add_field_method_get("inventory", |_, this| Ok(LuaInventory(this.0.clone())));
        fields.add_field_method_get("status", |_, this| {
            let s = match this.0.peer_status() {
                PeerStatus::FetchingServerData => "FetchingServerData",
                PeerStatus::ConnectingToServer => "ConnectingToServer",
                PeerStatus::InGame => "InGame",
                PeerStatus::InWorld => "InWorld",
            };
            Ok(s.to_string())
        });
        fields.add_field_method_get("ping", |_, this| Ok(this.0.runtime.ping()));
        fields.add_field_method_get("isInWorld", |_, this| {
            let in_world = match this.0.world.data.try_lock() {
                Ok(w) => w.name != "EXIT",
                Err(_) => false,
            };
            Ok(in_world)
        });
    }
}

// ── Lua UserData: Position ──────────────────────────────────────

pub struct LuaPosition(pub f32, pub f32);

impl UserData for LuaPosition {
    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("x", |_, this, ()| Ok(this.0));
        methods.add_method("y", |_, this, ()| Ok(this.1));
        methods.add_method("tileX", |_, this, ()| Ok((this.0 / 32.0).floor() as i32));
        methods.add_method("tileY", |_, this, ()| Ok((this.1 / 32.0).floor() as i32));
    }
}

// ── Lua UserData: GamePacket ────────────────────────────────────

pub struct LuaGamePacket(pub NetGamePacketData);

impl mlua::FromLua for LuaGamePacket {
    fn from_lua(value: mlua::Value, _lua: &Lua) -> mlua::Result<Self> {
        match value {
            mlua::Value::UserData(ud) => {
                let pkt = ud.borrow::<LuaGamePacket>()?;
                Ok(LuaGamePacket(NetGamePacketData {
                    _type: pkt.0._type,
                    object_type: pkt.0.object_type,
                    jump_count: pkt.0.jump_count,
                    animation_type: pkt.0.animation_type,
                    net_id: pkt.0.net_id,
                    target_net_id: pkt.0.target_net_id,
                    flags: pkt.0.flags,
                    float_variable: pkt.0.float_variable,
                    value: pkt.0.value,
                    vector_x: pkt.0.vector_x,
                    vector_y: pkt.0.vector_y,
                    vector_x2: pkt.0.vector_x2,
                    vector_y2: pkt.0.vector_y2,
                    particle_rotation: pkt.0.particle_rotation,
                    int_x: pkt.0.int_x,
                    int_y: pkt.0.int_y,
                    extended_data_length: pkt.0.extended_data_length,
                }))
            }
            _ => Err(mlua::Error::FromLuaConversionError {
                from: value.type_name(),
                to: "GamePacket".to_string(),
                message: Some("expected GamePacket userdata".to_string()),
            }),
        }
    }
}

impl UserData for LuaGamePacket {
    fn add_fields<F: UserDataFields<Self>>(fields: &mut F) {
        fields.add_field_method_get("type", |_, this| Ok(this.0._type as u8));
        fields.add_field_method_set("type", |_, this, v: u8| {
            this.0._type = NetGamePacket::from(v);
            Ok(())
        });
        fields.add_field_method_get("objectType", |_, this| Ok(this.0.object_type));
        fields.add_field_method_set("objectType", |_, this, v: u8| {
            this.0.object_type = v;
            Ok(())
        });
        fields.add_field_method_get("jumpCount", |_, this| Ok(this.0.jump_count));
        fields.add_field_method_set("jumpCount", |_, this, v: u8| {
            this.0.jump_count = v;
            Ok(())
        });
        fields.add_field_method_get("animationType", |_, this| Ok(this.0.animation_type));
        fields.add_field_method_set("animationType", |_, this, v: u8| {
            this.0.animation_type = v;
            Ok(())
        });
        fields.add_field_method_get("netId", |_, this| Ok(this.0.net_id));
        fields.add_field_method_set("netId", |_, this, v: u32| {
            this.0.net_id = v;
            Ok(())
        });
        fields.add_field_method_get("targetNetId", |_, this| Ok(this.0.target_net_id));
        fields.add_field_method_set("targetNetId", |_, this, v: i32| {
            this.0.target_net_id = v;
            Ok(())
        });
        fields.add_field_method_get("flags", |_, this| Ok(this.0.flags.bits()));
        fields.add_field_method_set("flags", |_, this, v: u32| {
            this.0.flags = crate::types::flags::PacketFlag::from_bits_truncate(v);
            Ok(())
        });
        fields.add_field_method_get("floatVar", |_, this| Ok(this.0.float_variable));
        fields.add_field_method_set("floatVar", |_, this, v: f32| {
            this.0.float_variable = v;
            Ok(())
        });
        fields.add_field_method_get("value", |_, this| Ok(this.0.value));
        fields.add_field_method_set("value", |_, this, v: u32| {
            this.0.value = v;
            Ok(())
        });
        fields.add_field_method_get("vecX", |_, this| Ok(this.0.vector_x));
        fields.add_field_method_set("vecX", |_, this, v: f32| {
            this.0.vector_x = v;
            Ok(())
        });
        fields.add_field_method_get("vecY", |_, this| Ok(this.0.vector_y));
        fields.add_field_method_set("vecY", |_, this, v: f32| {
            this.0.vector_y = v;
            Ok(())
        });
        fields.add_field_method_get("vecX2", |_, this| Ok(this.0.vector_x2));
        fields.add_field_method_set("vecX2", |_, this, v: f32| {
            this.0.vector_x2 = v;
            Ok(())
        });
        fields.add_field_method_get("vecY2", |_, this| Ok(this.0.vector_y2));
        fields.add_field_method_set("vecY2", |_, this, v: f32| {
            this.0.vector_y2 = v;
            Ok(())
        });
        fields.add_field_method_get("intX", |_, this| Ok(this.0.int_x));
        fields.add_field_method_set("intX", |_, this, v: i32| {
            this.0.int_x = v;
            Ok(())
        });
        fields.add_field_method_get("intY", |_, this| Ok(this.0.int_y));
        fields.add_field_method_set("intY", |_, this, v: i32| {
            this.0.int_y = v;
            Ok(())
        });
        fields.add_field_method_get("extDataLength", |_, this| Ok(this.0.extended_data_length));
        fields.add_field_method_set("extDataLength", |_, this, v: u32| {
            this.0.extended_data_length = v;
            Ok(())
        });
    }
}

// ── Lua UserData: Inventory ─────────────────────────────────────

pub struct LuaInventory(pub Arc<Bot>);

impl Clone for LuaInventory {
    fn clone(&self) -> Self {
        LuaInventory(self.0.clone())
    }
}

impl UserData for LuaInventory {
    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("getItemCount", |_, this, id: u32| {
            Ok(this.0.inventory.get_item_count(id as u16) as u32)
        });
        methods.add_method("hasItem", |_, this, (id, count): (u32, Option<u8>)| {
            Ok(this.0.inventory.has_item(id as u16, count.unwrap_or(1)))
        });
        methods.add_method("getItems", |lua, this, ()| {
            let items = this.0.inventory.get_all_items();
            let table = lua.create_table()?;
            for (i, (id, item)) in items.iter().enumerate() {
                let entry = lua.create_table()?;
                entry.set("id", *id as u32)?;
                entry.set("amount", item.amount as u32)?;
                table.set(i + 1, entry)?;
            }
            Ok(table)
        });
        methods.add_method("getSize", |_, this, ()| {
            let (size, _) = this.0.inventory.size_and_count();
            Ok(size)
        });
        methods.add_method("getCount", |_, this, ()| {
            let (_, count) = this.0.inventory.size_and_count();
            Ok(count as u32)
        });
        methods.add_method("isFull", |_, this, ()| {
            let (size, count) = this.0.inventory.size_and_count();
            Ok(count as u32 >= size)
        });
        methods.add_method("findItem", |lua, this, id: u32| {
            let count = this.0.inventory.get_item_count(id as u16);
            if count == 0 {
                Ok(mlua::Value::Nil)
            } else {
                let entry = lua.create_table()?;
                entry.set("id", id)?;
                entry.set("amount", count as u32)?;
                Ok(mlua::Value::Table(entry))
            }
        });
    }

    fn add_fields<F: UserDataFields<Self>>(fields: &mut F) {
        fields.add_field_method_get("gems", |_, this| Ok(this.0.inventory.gems()));
    }
}

// ── Lua UserData: World ─────────────────────────────────────────

pub struct LuaWorld(pub Arc<Bot>);

impl Clone for LuaWorld {
    fn clone(&self) -> Self {
        LuaWorld(self.0.clone())
    }
}

impl UserData for LuaWorld {
    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("getTile", |_, this, (x, y): (u32, u32)| {
            let world = this.0.world.data.lock().unwrap();
            if let Some(tile) = world.get_tile(x, y) {
                let is_seed = matches!(tile.tile_type, gtworld_r::TileType::Seed { .. });
                let has_lock = matches!(tile.tile_type, gtworld_r::TileType::Lock { .. });
                let collision_type = {
                    let db = this.0.world.item_database.read().unwrap();
                    db.get_item(&(tile.foreground_item_id as u32))
                        .map(|i| i.collision_type)
                        .unwrap_or(0)
                };
                Ok(Some(LuaTile {
                    x: tile.x,
                    y: tile.y,
                    foreground: tile.foreground_item_id,
                    background: tile.background_item_id,
                    collision_type,
                    is_seed,
                    has_lock,
                }))
            } else {
                Ok(None)
            }
        });
        methods.add_method("getTiles", |_, this, ()| {
            let world = this.0.world.data.lock().unwrap();
            let db = this.0.world.item_database.read().unwrap();
            let tiles: Vec<LuaTile> = world
                .tiles
                .iter()
                .map(|tile| {
                    let is_seed = matches!(tile.tile_type, gtworld_r::TileType::Seed { .. });
                    let has_lock = matches!(tile.tile_type, gtworld_r::TileType::Lock { .. });
                    let collision_type = db
                        .get_item(&(tile.foreground_item_id as u32))
                        .map(|i| i.collision_type)
                        .unwrap_or(0);
                    LuaTile {
                        x: tile.x,
                        y: tile.y,
                        foreground: tile.foreground_item_id,
                        background: tile.background_item_id,
                        collision_type,
                        is_seed,
                        has_lock,
                    }
                })
                .collect();
            Ok(tiles)
        });
        methods.add_method("getPlayers", |lua, this, ()| {
            let players = this.0.world.players.lock().unwrap();
            let table = lua.create_table()?;
            for (i, (_, player)) in players.iter().enumerate() {
                table.set(i + 1, LuaPlayer {
                    name: player.name.clone(),
                    net_id: player.net_id,
                    user_id: player.user_id,
                    country: player.country.clone(),
                    pos_x: player.position.0,
                    pos_y: player.position.1,
                    invisible: player.invisible,
                    is_mod: player.m_state == 1,
                })?;
            }
            Ok(table)
        });
        methods.add_method("getPlayer", |_, this, net_id: u32| {
            let players = this.0.world.players.lock().unwrap();
            Ok(players.get(&net_id).map(|p| LuaPlayer {
                name: p.name.clone(),
                net_id: p.net_id,
                user_id: p.user_id,
                country: p.country.clone(),
                pos_x: p.position.0,
                pos_y: p.position.1,
                invisible: p.invisible,
                is_mod: p.m_state == 1,
            }))
        });
        methods.add_method("getDroppedItems", |lua, this, ()| {
            let world = this.0.world.data.lock().unwrap();
            let table = lua.create_table()?;
            for (i, item) in world.dropped.items.iter().enumerate() {
                let entry = lua.create_table()?;
                entry.set("uid", item.uid)?;
                entry.set("id", item.id as u32)?;
                entry.set("x", item.x)?;
                entry.set("y", item.y)?;
                entry.set("count", item.count as u32)?;
                table.set(i + 1, entry)?;
            }
            Ok(table)
        });
        methods.add_method("isInWorld", |_, this, ()| {
            let world = this.0.world.data.lock().unwrap();
            Ok(world.name != "EXIT")
        });
    }

    fn add_fields<F: UserDataFields<Self>>(fields: &mut F) {
        fields.add_field_method_get("name", |_, this| {
            let world = this.0.world.data.lock().unwrap();
            Ok(world.name.clone())
        });
        fields.add_field_method_get("width", |_, this| {
            let world = this.0.world.data.lock().unwrap();
            Ok(world.width)
        });
        fields.add_field_method_get("height", |_, this| {
            let world = this.0.world.data.lock().unwrap();
            Ok(world.height)
        });
    }
}

// ── Lua UserData: Tile ───────────────────────────────

pub struct LuaTile {
    pub x: u32,
    pub y: u32,
    pub foreground: u16,
    pub background: u16,
    pub collision_type: u8,
    pub is_seed: bool,
    pub has_lock: bool,
}

impl UserData for LuaTile {
    fn add_fields<F: UserDataFields<Self>>(fields: &mut F) {
        fields.add_field_method_get("x", |_, this| Ok(this.x));
        fields.add_field_method_get("y", |_, this| Ok(this.y));
        fields.add_field_method_get("foreground", |_, this| Ok(this.foreground as u32));
        fields.add_field_method_get("background", |_, this| Ok(this.background as u32));
        fields.add_field_method_get("isCollidable", |_, this| {
            Ok(this.collision_type == 1 || this.collision_type == 6)
        });
        fields.add_field_method_get("collisionType", |_, this| Ok(this.collision_type));
        fields.add_field_method_get("hasLock", |_, this| Ok(this.has_lock));
        fields.add_field_method_get("isSeed", |_, this| Ok(this.is_seed));
    }
}

// ── Lua UserData: Player ─────────────────────────────

#[derive(Clone)]
pub struct LuaPlayer {
    pub name: String,
    pub net_id: u32,
    pub user_id: u32,
    pub country: String,
    pub pos_x: f32,
    pub pos_y: f32,
    pub invisible: bool,
    pub is_mod: bool,
}

impl UserData for LuaPlayer {
    fn add_fields<F: UserDataFields<Self>>(fields: &mut F) {
        fields.add_field_method_get("name", |_, this| Ok(this.name.clone()));
        fields.add_field_method_get("netId", |_, this| Ok(this.net_id));
        fields.add_field_method_get("userId", |_, this| Ok(this.user_id));
        fields.add_field_method_get("country", |_, this| Ok(this.country.clone()));
        fields.add_field_method_get("pos", |_, this| Ok(LuaPosition(this.pos_x, this.pos_y)));
        fields.add_field_method_get("invisible", |_, this| Ok(this.invisible));
        fields.add_field_method_get("isMod", |_, this| Ok(this.is_mod));
    }
}
