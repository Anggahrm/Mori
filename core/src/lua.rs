use std::sync::Arc;

use crate::types::bot::{BotArc, LuaGamePacket};
use crate::types::net_game_packet::{NetGamePacket, NetGamePacketData};
use crate::Bot;

pub fn initialize(bot: &Arc<Bot>) {
    let bot_arc = BotArc(bot.clone());
    let lua = &bot.scripting.lua;

    // getBot() -> Bot
    let get_bot = lua
        .create_function(move |_, ()| Ok(bot_arc.clone()))
        .unwrap();
    lua.globals().set("getBot", get_bot).unwrap();

    // sleep(ms)
    let sleep = lua
        .create_function(move |_, duration: u64| {
            std::thread::sleep(std::time::Duration::from_millis(duration));
            Ok(())
        })
        .unwrap();
    lua.globals().set("sleep", sleep).unwrap();

    // log(message)
    let log_bot = bot.clone();
    let log_fn = lua
        .create_function(move |_, msg: String| {
            log_bot.runtime.push_log(msg);
            Ok(())
        })
        .unwrap();
    lua.globals().set("log", log_fn).unwrap();

    // getItemInfo(id) -> table|nil
    let info_bot = bot.clone();
    let get_item_info = lua
        .create_function(move |lua, id: u32| {
            let db = info_bot.world.item_database.read().unwrap();
            match db.get_item(&id) {
                Some(item) => {
                    let t = lua.create_table()?;
                    t.set("id", item.id)?;
                    t.set("name", item.name.clone())?;
                    t.set("rarity", item.rarity)?;
                    t.set("collisionType", item.collision_type)?;
                    t.set("actionType", item.action_type)?;
                    Ok(mlua::Value::Table(t))
                }
                None => Ok(mlua::Value::Nil),
            }
        })
        .unwrap();
    lua.globals().set("getItemInfo", get_item_info).unwrap();

    // getItemInfoByName(name) -> table|nil
    let info_name_bot = bot.clone();
    let get_item_info_by_name = lua
        .create_function(move |lua, name: String| {
            let db = info_name_bot.world.item_database.read().unwrap();
            let found = db.items.values().find(|item| item.name == name);
            match found {
                Some(item) => {
                    let t = lua.create_table()?;
                    t.set("id", item.id)?;
                    t.set("name", item.name.clone())?;
                    t.set("rarity", item.rarity)?;
                    t.set("collisionType", item.collision_type)?;
                    t.set("actionType", item.action_type)?;
                    Ok(mlua::Value::Table(t))
                }
                None => Ok(mlua::Value::Nil),
            }
        })
        .unwrap();
    lua.globals()
        .set("getItemInfoByName", get_item_info_by_name)
        .unwrap();

    // GamePacket(type?) -> GamePacket
    let game_packet_ctor = lua
        .create_function(move |_, pkt_type: Option<u8>| {
            let mut pkt = NetGamePacketData::default();
            if let Some(t) = pkt_type {
                pkt._type = NetGamePacket::from(t);
            }
            Ok(LuaGamePacket(pkt))
        })
        .unwrap();
    lua.globals().set("GamePacket", game_packet_ctor).unwrap();
}

/// Invokes all registered Lua callbacks for the given event name with the provided arguments.
/// Removes one-shot callbacks after invocation.
pub fn invoke_callbacks<A: mlua::IntoLuaMulti + Clone>(bot: &Bot, event: &str, args: A) {
    let lua = &bot.scripting.lua;
    let mut cbs = bot.scripting.callbacks.lock().unwrap();

    if let Some(callbacks) = cbs.get_mut(event) {
        let mut to_remove = Vec::new();

        for (i, cb) in callbacks.iter().enumerate() {
            if let Ok(func) = lua.registry_value::<mlua::Function>(&cb.key) {
                if let Err(e) = func.call::<()>(args.clone()) {
                    bot.runtime
                        .push_log(format!("[Lua] Error in '{}' callback: {}", event, e));
                }
                if cb.once {
                    to_remove.push(i);
                }
            }
        }

        // Remove once-callbacks in reverse order to maintain indices
        for i in to_remove.into_iter().rev() {
            let removed = callbacks.remove(i);
            let _ = lua.remove_registry_value(removed.key);
        }

        if callbacks.is_empty() {
            cbs.remove(event);
        }
    }
}

/// Check if there are any registered callbacks for an event (avoids unnecessary work).
pub fn has_callbacks(bot: &Bot, event: &str) -> bool {
    let cbs = bot.scripting.callbacks.lock().unwrap();
    cbs.get(event).is_some_and(|v| !v.is_empty())
}
