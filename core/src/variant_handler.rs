use crate::lua;
use crate::types::bot::LuaPlayer;
use crate::types::net_message::NetMessage;
use crate::types::player::Player;
use crate::types::status::PeerStatus;
use crate::utils::proton::HashMode;
use crate::utils::variant::VariantList;
use crate::{Bot, utils};
use std::collections::HashMap;
use std::fs;
use std::sync::Arc;

pub fn handle(bot: &Arc<Bot>, data: &[u8]) {
    let variant = VariantList::deserialize(&data).expect("Failed to deserialize variant list");
    let function_call: String = variant.get(0).unwrap().as_string();

    println!("Function call: {}", function_call);

    // Fire onVariant callback with variant list as Lua table
    if lua::has_callbacks(bot, "onVariant") {
        let lua = &bot.scripting.lua;
        if let Ok(table) = variant_list_to_lua_table(lua, &variant) {
            lua::invoke_callbacks(bot, "onVariant", table);
        }
    }

    match function_call.as_str() {
        "OnSendToServer" => {
            let port = variant.get(1).unwrap().as_int32();
            let token = variant.get(2).unwrap().as_int32();
            let user_id = variant.get(3).unwrap().as_int32();
            let server_data = variant.get(4).unwrap().as_string();
            let parsed_server_data: Vec<String> = server_data
                .split('|')
                .map(|s| s.trim_end().to_string())
                .collect();
            let aat = variant.get(5).unwrap().as_int32();

            let mut server_data_lock = bot.auth.server_data();
            let server_data = server_data_lock.as_mut().unwrap();

            server_data.server = parsed_server_data[0].clone();
            server_data.port = port as u16;

            bot.runtime.set_redirecting(true);

            let mut login_info_lock = bot.auth.login_info();
            let login_info = login_info_lock.as_mut().unwrap();

            login_info.token = token.to_string();
            login_info.user = user_id.to_string();
            login_info.door_id = parsed_server_data[1].clone();
            login_info.uuid = parsed_server_data[2].clone();
            login_info.aat = aat.to_string();

            bot.disconnect()
        }
        "OnSuperMainStartAcceptLogonHrdxs47254722215a" => {
            let server_hash = variant.get(1).unwrap().as_uint32();

            match fs::read("items.dat") {
                Ok(data) => {
                    let hash = utils::proton::hash(
                        data.as_slice(),
                        HashMode::FixedLength(data.len() as i32),
                    ) as u32;

                    if hash == server_hash {
                        bot.send_text_packet(
                            NetMessage::GenericText,
                            b"action|enter_game\n",
                        );
                        bot.runtime.set_redirecting(false);
                        let item_database = gtitem_r::load_from_file("items.dat")
                            .expect("Failed to load items.dat");
                        let mut item_database_lock = bot.world.item_database.write().unwrap();
                        *item_database_lock = item_database;

                        {
                            let mut peer_status = bot.peer_status.lock().unwrap();
                            *peer_status = PeerStatus::InGame;
                        }

                        return;
                    }
                }
                Err(_) => {
                    println!("Fetching server items.dat...");
                }
            }

            bot.send_text_packet(
                NetMessage::GenericText,
                b"action|refresh_item_data\n",
            );
        }
        "OnSetPos" => {
            let pos = variant.get(1).unwrap().as_vec2();
            bot.movement.set_position(pos.0, pos.1);

            lua::invoke_callbacks(bot, "onSetPos", (pos.0, pos.1));
        }
        "OnTalkBubble" => {
            let net_id_val = variant.get(1).unwrap().as_int32();
            let message = variant.get(2).unwrap().as_string();
            println!("[TALK] {}", message);

            lua::invoke_callbacks(bot, "onChat", (net_id_val, message.clone()));
        }
        "OnConsoleMessage" => {
            let message = variant.get(1).unwrap().as_string();
            println!("[CONSOLE] {}", message);

            lua::invoke_callbacks(bot, "onConsole", message);
        }
        "OnSetBux" => {
            let gems = variant.get(1).unwrap().as_int32();
            bot.inventory.add_gems(gems);
        }
        "SetHasGrowID" => {
            let growid = variant.get(2).unwrap().as_string();
            let mut login_info_lock = bot.auth.login_info();
            let login_info = login_info_lock.as_mut().unwrap();
            login_info.tank_id_name = growid;
        }
        "OnRemove" => {
            let message = variant.get(1).unwrap().as_string();
            let data = parse_and_store_as_map(&message);
            let net_id: u32 = data["netID"].parse().unwrap();

            let mut players = bot.world.players.lock().unwrap();
            players.remove(&net_id);
            drop(players);

            lua::invoke_callbacks(bot, "onPlayerLeave", net_id);
        }
        "OnSpawn" => {
            let message = variant.get(1).unwrap().as_string();
            let data = parse_and_store_as_map(&message);

            if data.contains_key("type") {
                bot.runtime.set_net_id(
                    data.get("netID")
                        .unwrap()
                        .parse()
                        .expect("Failed to parse netid"),
                );
                bot.runtime.set_user_id(
                    data.get("userID")
                        .unwrap()
                        .parse()
                        .expect("Failed to parse userID"),
                );
            } else {
                let player = Player {
                    _type: data.get("spawn").unwrap_or(&String::new()).clone(),
                    avatar: data.get("avatar").unwrap_or(&String::new()).clone(),
                    net_id: data["netID"].parse().expect("Failed to parse netid"),
                    online_id: data.get("onlineID").unwrap_or(&String::new()).clone(),
                    e_id: data["eid"].clone(),
                    ip: data["ip"].clone(),
                    col_rect: data["colrect"].clone(),
                    title_icon: data.get("titleIcon").unwrap_or(&String::new()).clone(),
                    m_state: data["mstate"].parse().expect("Failed to parse mstate"),
                    user_id: data["userID"].parse().expect("Failed to parse userid"),
                    invisible: data
                        .get("invis")
                        .unwrap_or(&"0".to_string())
                        .parse::<u32>()
                        .expect("Failed to parse invisible")
                        != 0,
                    name: data["name"].clone(),
                    country: data["country"].clone(),
                    position: {
                        if data.contains_key("posXY") {
                            let pos_xy = data
                                .get("posXY")
                                .unwrap()
                                .split('|')
                                .map(|s| {
                                    s.trim().parse().expect("Fail to parse player coordinates")
                                })
                                .collect::<Vec<f32>>();
                            (pos_xy[0], pos_xy[1])
                        } else {
                            (0.0, 0.0)
                        }
                    },
                };

                if player.m_state == 1 || player.invisible {
                    bot.leave();
                }

                // Fire onPlayerJoin before inserting
                lua::invoke_callbacks(bot, "onPlayerJoin", LuaPlayer {
                    name: player.name.clone(),
                    net_id: player.net_id,
                    user_id: player.user_id,
                    country: player.country.clone(),
                    pos_x: player.position.0,
                    pos_y: player.position.1,
                    invisible: player.invisible,
                    is_mod: player.m_state == 1,
                });

                let mut players = bot.world.players.lock().unwrap();
                players.insert(player.net_id, player);
            }
        }
        "OnDialogRequest" => {
            let message = variant.get(1).unwrap().as_string();

            lua::invoke_callbacks(bot, "onDialogRequest", message.clone());

            let cb = {
                let dialog_callback = bot.temporary_data.dialog_callback.lock().unwrap();
                dialog_callback.clone()
            };

            if let Some(cb) = cb {
                cb(bot);
            }

            if message.contains("Gazette") {
                bot.send_text_packet(
                    NetMessage::GenericText,
                    b"action|dialog_return\ndialog_name|gazette\nbuttonClicked|banner\n",
                );
            }
        }
        _ => {}
    }
}

fn variant_list_to_lua_table(
    lua: &mlua::Lua,
    variant: &VariantList,
) -> mlua::Result<mlua::Table> {
    let table = lua.create_table()?;
    let mut i = 0;
    while let Some(v) = variant.get(i) {
        {
            match v {
                crate::utils::variant::Variant::String(s) => {
                    table.set(i + 1, s.clone())?;
                }
                crate::utils::variant::Variant::Float(f) => {
                    table.set(i + 1, *f)?;
                }
                crate::utils::variant::Variant::Unsigned(u) => {
                    table.set(i + 1, *u)?;
                }
                crate::utils::variant::Variant::Signed(s) => {
                    table.set(i + 1, *s)?;
                }
                crate::utils::variant::Variant::Vec2(xy) => {
                    let v = lua.create_table()?;
                    v.set("x", xy.0)?;
                    v.set("y", xy.1)?;
                    table.set(i + 1, v)?;
                }
                crate::utils::variant::Variant::Vec3(xyz) => {
                    let v = lua.create_table()?;
                    v.set("x", xyz.0)?;
                    v.set("y", xyz.1)?;
                    v.set("z", xyz.2)?;
                    table.set(i + 1, v)?;
                }
                _ => {
                    table.set(i + 1, mlua::Value::Nil)?;
                }
            }
        }
        i += 1;
    }
    Ok(table)
}

fn parse_and_store_as_map(input: &str) -> HashMap<String, String> {
    let mut map = HashMap::new();
    for line in input.lines() {
        let parts: Vec<&str> = line.split('|').collect();
        if parts.len() >= 2 {
            let key = parts[0].to_string();
            let value = parts[1..].join("|");
            map.insert(key, value);
        }
    }
    map
}
