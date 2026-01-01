use axum::{
    Json,
    Router,
    extract::{Path, State},
    http::StatusCode,
    response::{Html, IntoResponse},
    routing::{delete, get, post},
};
use gt_core::gtitem_r::load_from_file;
use gt_core::gtitem_r::structs::ItemDatabase;
use gt_core::types::bot::LoginVia;
use gt_core::{Bot, PrivateServerConfig, Socks5Config};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;
use std::net::SocketAddr;
use std::sync::{Arc, RwLock};
use std::thread::JoinHandle;
use tokio::net::TcpListener;
use tower_http::cors::{Any, CorsLayer};
use tower_http::services::ServeDir;
use uuid::Uuid;

mod web;

/// Application state shared across all handlers
struct AppState {
    bots: RwLock<HashMap<Uuid, (Arc<Bot>, JoinHandle<()>)>>,
    items_database: Arc<RwLock<ItemDatabase>>,
}

impl AppState {
    fn new() -> Self {
        // Try to load items.dat, but don't fail if it doesn't exist
        let item_database = match load_from_file("items.dat") {
            Ok(db) => db,
            Err(_) => {
                println!("Warning: items.dat not found, using empty item database");
                ItemDatabase::default()
            }
        };
        
        Self {
            bots: RwLock::new(HashMap::new()),
            items_database: Arc::new(RwLock::new(item_database)),
        }
    }
}

// Request/Response types
#[derive(Deserialize)]
struct CreateBotRequest {
    login_method: String,
    credentials: Option<Credentials>,
    socks5: Option<String>,
    // Private Server fields
    private_server: Option<PrivateServerRequest>,
}

#[derive(Deserialize)]
struct PrivateServerRequest {
    server_ip: String,
    server_port: u16,
    server_data_url: Option<String>,
    use_https: Option<bool>,
}

#[derive(Deserialize)]
struct Credentials {
    growid: Option<String>,
    password: Option<String>,
    token: Option<String>,
}

#[derive(Serialize)]
struct ApiResponse<T> {
    success: bool,
    data: Option<T>,
    error: Option<String>,
}

#[derive(Serialize)]
struct BotInfo {
    id: String,
    name: String,
    status: String,
    gems: i32,
    ping: u32,
    world: Option<String>,
    is_private_server: bool,
}

#[derive(Serialize)]
struct BotListResponse {
    bots: Vec<BotInfo>,
}

#[derive(Serialize)]
struct InventoryResponse {
    size: usize,
    item_count: usize,
    items: Vec<InventoryItem>,
}

#[derive(Serialize)]
struct InventoryItem {
    id: u16,
    name: String,
    amount: u16,
}

#[derive(Serialize)]
struct WorldResponse {
    name: String,
    width: u32,
    height: u32,
    players: Vec<PlayerInfo>,
}

#[derive(Serialize)]
struct PlayerInfo {
    name: String,
    net_id: u32,
    position: (f32, f32),
}

#[derive(Serialize)]
struct LogsResponse {
    logs: Vec<String>,
}

#[derive(Deserialize)]
struct WarpRequest {
    world_name: String,
}

#[derive(Deserialize)]
struct SayRequest {
    message: String,
}

#[derive(Deserialize)]
struct MoveRequest {
    direction: String,
    tiles: Option<i32>,
}

#[tokio::main]
async fn main() {
    println!("Starting Mori Web Server...");
    
    let state = Arc::new(AppState::new());
    
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);
    
    let app = Router::new()
        // Serve static files and templates
        .route("/", get(index_handler))
        // API routes
        .route("/api/bots", get(list_bots))
        .route("/api/bots", post(create_bot))
        .route("/api/bots/{id}", delete(remove_bot))
        .route("/api/bots/{id}/inventory", get(get_inventory))
        .route("/api/bots/{id}/world", get(get_world))
        .route("/api/bots/{id}/logs", get(get_logs))
        .route("/api/bots/{id}/warp", post(warp_bot))
        .route("/api/bots/{id}/say", post(say_message))
        .route("/api/bots/{id}/move", post(move_bot))
        .route("/api/bots/{id}/connect", post(connect_bot))
        .route("/api/bots/{id}/disconnect", post(disconnect_bot))
        .route("/api/bots/{id}/collect", post(collect_items))
        .route("/api/bots/{id}/leave", post(leave_world))
        // Static files
        .nest_service("/static", ServeDir::new("static"))
        .layer(cors)
        .with_state(state);
    
    // Get port from environment (Heroku sets PORT)
    let port: u16 = env::var("PORT")
        .unwrap_or_else(|_| "3000".to_string())
        .parse()
        .expect("PORT must be a number");
    
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    println!("Mori Web UI running on http://{}", addr);
    
    let listener = TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn index_handler() -> impl IntoResponse {
    Html(include_str!("../templates/index.html"))
}

async fn list_bots(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let bots = state.bots.read().unwrap();
    let mut bot_list = Vec::new();
    
    for (id, (bot, _)) in bots.iter() {
        let name = bot.auth.try_login_info()
            .and_then(|guard| guard.as_ref().map(|info| info.tank_id_name.clone()))
            .unwrap_or_else(|| "Connecting...".to_string());
        
        let status = format!("{:?}", bot.enet_status());
        let gems = bot.inventory.gems();
        let ping = bot.runtime.ping();
        let world = bot.world.data.try_lock()
            .ok()
            .map(|w| if w.name != "EXIT" { Some(w.name.clone()) } else { None })
            .flatten();
        
        bot_list.push(BotInfo {
            id: id.to_string(),
            name,
            status,
            gems,
            ping,
            world,
            is_private_server: bot.is_private_server(),
        });
    }
    
    Json(ApiResponse {
        success: true,
        data: Some(BotListResponse { bots: bot_list }),
        error: None,
    })
}

async fn create_bot(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateBotRequest>,
) -> impl IntoResponse {
    // Parse login method
    let login_via = match req.login_method.as_str() {
        "legacy" => {
            if let Some(creds) = &req.credentials {
                LoginVia::LEGACY([
                    creds.growid.clone().unwrap_or_default(),
                    creds.password.clone().unwrap_or_default(),
                ])
            } else {
                return (StatusCode::BAD_REQUEST, Json(ApiResponse::<serde_json::Value> {
                    success: false,
                    data: None,
                    error: Some("Legacy login requires growid and password".to_string()),
                }));
            }
        }
        "ltoken" => {
            if let Some(creds) = &req.credentials {
                if let Some(token) = &creds.token {
                    let parts: Vec<&str> = token.split(':').collect();
                    if parts.len() == 4 {
                        LoginVia::LTOKEN([
                            parts[0].to_string(),
                            parts[1].to_string(),
                            parts[2].to_string(),
                            parts[3].to_string(),
                        ])
                    } else {
                        return (StatusCode::BAD_REQUEST, Json(ApiResponse::<serde_json::Value> {
                            success: false,
                            data: None,
                            error: Some("LTOKEN must have 4 values separated by ':'".to_string()),
                        }));
                    }
                } else {
                    return (StatusCode::BAD_REQUEST, Json(ApiResponse::<serde_json::Value> {
                        success: false,
                        data: None,
                        error: Some("LTOKEN login requires token".to_string()),
                    }));
                }
            } else {
                return (StatusCode::BAD_REQUEST, Json(ApiResponse::<serde_json::Value> {
                    success: false,
                    data: None,
                    error: Some("LTOKEN login requires credentials".to_string()),
                }));
            }
        }
        "google" => LoginVia::GOOGLE,
        "apple" => LoginVia::APPLE,
        _ => LoginVia::LEGACY([String::new(), String::new()]),
    };
    
    // Parse SOCKS5 proxy
    let socks5_config = req.socks5.as_ref().and_then(|s| {
        if s.is_empty() {
            return None;
        }
        let parts: Vec<&str> = s.split(':').collect();
        match parts.len() {
            2 => {
                let addr = format!("{}:{}", parts[0], parts[1]).parse().ok()?;
                Some(Socks5Config {
                    proxy_addr: addr,
                    username: None,
                    password: None,
                })
            }
            4 => {
                let addr = format!("{}:{}", parts[0], parts[1]).parse().ok()?;
                Some(Socks5Config {
                    proxy_addr: addr,
                    username: Some(parts[2].to_string()),
                    password: Some(parts[3].to_string()),
                })
            }
            _ => None,
        }
    });
    
    // Parse Private Server config
    let private_server_config = req.private_server.as_ref().map(|ps| {
        PrivateServerConfig {
            server_ip: ps.server_ip.clone(),
            server_port: ps.server_port,
            server_data_url: ps.server_data_url.clone().unwrap_or_else(|| ps.server_ip.clone()),
            use_https: ps.use_https.unwrap_or(false),
            skip_login_url: true,
        }
    });
    
    let items_database = state.items_database.clone();
    let bot_id = Uuid::new_v4();
    
    // Create bot with private server support
    let (bot, _receiver) = Bot::new_with_ps(
        login_via,
        None, // No token fetcher for web (would need headless browser)
        items_database,
        socks5_config,
        private_server_config,
    );
    
    let bot_clone = bot.clone();
    let handle = std::thread::spawn(move || {
        bot_clone.logon(None);
    });
    
    state.bots.write().unwrap().insert(bot_id, (bot, handle));
    
    (StatusCode::OK, Json(ApiResponse {
        success: true,
        data: Some(serde_json::json!({ "id": bot_id.to_string() })),
        error: None,
    }))
}

async fn remove_bot(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let uuid = match Uuid::parse_str(&id) {
        Ok(u) => u,
        Err(_) => {
            return Json(ApiResponse::<()> {
                success: false,
                data: None,
                error: Some("Invalid bot ID".to_string()),
            });
        }
    };
    
    let removed = state.bots.write().unwrap().remove(&uuid);
    
    if removed.is_some() {
        Json(ApiResponse {
            success: true,
            data: Some(()),
            error: None,
        })
    } else {
        Json(ApiResponse::<()> {
            success: false,
            data: None,
            error: Some("Bot not found".to_string()),
        })
    }
}

async fn get_inventory(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let uuid = match Uuid::parse_str(&id) {
        Ok(u) => u,
        Err(_) => {
            return Json(ApiResponse::<InventoryResponse> {
                success: false,
                data: None,
                error: Some("Invalid bot ID".to_string()),
            });
        }
    };
    
    let bots = state.bots.read().unwrap();
    let bot = match bots.get(&uuid) {
        Some((b, _)) => b.clone(),
        None => {
            return Json(ApiResponse::<InventoryResponse> {
                success: false,
                data: None,
                error: Some("Bot not found".to_string()),
            });
        }
    };
    drop(bots);
    
    let snapshot = match bot.inventory.try_get_snapshot() {
        Some(s) => s,
        None => {
            return Json(ApiResponse::<InventoryResponse> {
                success: false,
                data: None,
                error: Some("Could not get inventory".to_string()),
            });
        }
    };
    
    let items_db = state.items_database.read().unwrap();
    let items: Vec<InventoryItem> = snapshot.item_amounts.iter()
        .map(|(id, amount)| {
            let name = items_db.items.get(&(*id as u32))
                .map(|item| item.name.clone())
                .unwrap_or_else(|| format!("Item #{}", id));
            InventoryItem {
                id: *id,
                name,
                amount: *amount as u16,
            }
        })
        .collect();
    
    Json(ApiResponse {
        success: true,
        data: Some(InventoryResponse {
            size: snapshot.size as usize,
            item_count: snapshot.item_count as usize,
            items,
        }),
        error: None,
    })
}

async fn get_world(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let uuid = match Uuid::parse_str(&id) {
        Ok(u) => u,
        Err(_) => {
            return Json(ApiResponse::<WorldResponse> {
                success: false,
                data: None,
                error: Some("Invalid bot ID".to_string()),
            });
        }
    };
    
    let bots = state.bots.read().unwrap();
    let bot = match bots.get(&uuid) {
        Some((b, _)) => b.clone(),
        None => {
            return Json(ApiResponse::<WorldResponse> {
                success: false,
                data: None,
                error: Some("Bot not found".to_string()),
            });
        }
    };
    drop(bots);
    
    let world_data = match bot.world.data.try_lock() {
        Ok(w) => w,
        Err(_) => {
            return Json(ApiResponse::<WorldResponse> {
                success: false,
                data: None,
                error: Some("Could not get world data".to_string()),
            });
        }
    };
    
    // Get players from the separate players HashMap
    let players_lock = match bot.world.players.try_lock() {
        Ok(p) => p,
        Err(_) => {
            return Json(ApiResponse::<WorldResponse> {
                success: false,
                data: None,
                error: Some("Could not get players data".to_string()),
            });
        }
    };
    
    let players: Vec<PlayerInfo> = players_lock.values()
        .map(|p| PlayerInfo {
            name: p.name.clone(),
            net_id: p.net_id,
            position: p.position,
        })
        .collect();
    
    Json(ApiResponse {
        success: true,
        data: Some(WorldResponse {
            name: world_data.name.clone(),
            width: world_data.width,
            height: world_data.height,
            players,
        }),
        error: None,
    })
}

async fn get_logs(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let uuid = match Uuid::parse_str(&id) {
        Ok(u) => u,
        Err(_) => {
            return Json(ApiResponse::<LogsResponse> {
                success: false,
                data: None,
                error: Some("Invalid bot ID".to_string()),
            });
        }
    };
    
    let bots = state.bots.read().unwrap();
    let bot = match bots.get(&uuid) {
        Some((b, _)) => b.clone(),
        None => {
            return Json(ApiResponse::<LogsResponse> {
                success: false,
                data: None,
                error: Some("Bot not found".to_string()),
            });
        }
    };
    drop(bots);
    
    let logs = bot.runtime.logs_snapshot();
    
    Json(ApiResponse {
        success: true,
        data: Some(LogsResponse { logs }),
        error: None,
    })
}

async fn warp_bot(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(req): Json<WarpRequest>,
) -> impl IntoResponse {
    let uuid = match Uuid::parse_str(&id) {
        Ok(u) => u,
        Err(_) => {
            return Json(ApiResponse::<()> {
                success: false,
                data: None,
                error: Some("Invalid bot ID".to_string()),
            });
        }
    };
    
    let bots = state.bots.read().unwrap();
    let bot = match bots.get(&uuid) {
        Some((b, _)) => b.clone(),
        None => {
            return Json(ApiResponse::<()> {
                success: false,
                data: None,
                error: Some("Bot not found".to_string()),
            });
        }
    };
    drop(bots);
    
    let world_name = req.world_name;
    std::thread::spawn(move || {
        bot.warp(world_name);
    });
    
    Json(ApiResponse {
        success: true,
        data: Some(()),
        error: None,
    })
}

async fn say_message(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(req): Json<SayRequest>,
) -> impl IntoResponse {
    let uuid = match Uuid::parse_str(&id) {
        Ok(u) => u,
        Err(_) => {
            return Json(ApiResponse::<()> {
                success: false,
                data: None,
                error: Some("Invalid bot ID".to_string()),
            });
        }
    };
    
    let bots = state.bots.read().unwrap();
    let bot = match bots.get(&uuid) {
        Some((b, _)) => b.clone(),
        None => {
            return Json(ApiResponse::<()> {
                success: false,
                data: None,
                error: Some("Bot not found".to_string()),
            });
        }
    };
    drop(bots);
    
    bot.say(&req.message);
    
    Json(ApiResponse {
        success: true,
        data: Some(()),
        error: None,
    })
}

async fn move_bot(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(req): Json<MoveRequest>,
) -> impl IntoResponse {
    let uuid = match Uuid::parse_str(&id) {
        Ok(u) => u,
        Err(_) => {
            return Json(ApiResponse::<()> {
                success: false,
                data: None,
                error: Some("Invalid bot ID".to_string()),
            });
        }
    };
    
    let bots = state.bots.read().unwrap();
    let bot = match bots.get(&uuid) {
        Some((b, _)) => b.clone(),
        None => {
            return Json(ApiResponse::<()> {
                success: false,
                data: None,
                error: Some("Bot not found".to_string()),
            });
        }
    };
    drop(bots);
    
    let tiles = req.tiles.unwrap_or(1);
    let (dx, dy) = match req.direction.as_str() {
        "up" => (0, -tiles),
        "down" => (0, tiles),
        "left" => (-tiles, 0),
        "right" => (tiles, 0),
        _ => {
            return Json(ApiResponse::<()> {
                success: false,
                data: None,
                error: Some("Invalid direction".to_string()),
            });
        }
    };
    
    std::thread::spawn(move || {
        bot.walk(dx, dy, false);
    });
    
    Json(ApiResponse {
        success: true,
        data: Some(()),
        error: None,
    })
}

async fn connect_bot(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let uuid = match Uuid::parse_str(&id) {
        Ok(u) => u,
        Err(_) => {
            return Json(ApiResponse::<()> {
                success: false,
                data: None,
                error: Some("Invalid bot ID".to_string()),
            });
        }
    };
    
    let bots = state.bots.read().unwrap();
    let bot = match bots.get(&uuid) {
        Some((b, _)) => b.clone(),
        None => {
            return Json(ApiResponse::<()> {
                success: false,
                data: None,
                error: Some("Bot not found".to_string()),
            });
        }
    };
    drop(bots);
    
    std::thread::spawn(move || {
        bot.connect_to_server();
    });
    
    Json(ApiResponse {
        success: true,
        data: Some(()),
        error: None,
    })
}

async fn disconnect_bot(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let uuid = match Uuid::parse_str(&id) {
        Ok(u) => u,
        Err(_) => {
            return Json(ApiResponse::<()> {
                success: false,
                data: None,
                error: Some("Invalid bot ID".to_string()),
            });
        }
    };
    
    let bots = state.bots.read().unwrap();
    let bot = match bots.get(&uuid) {
        Some((b, _)) => b.clone(),
        None => {
            return Json(ApiResponse::<()> {
                success: false,
                data: None,
                error: Some("Bot not found".to_string()),
            });
        }
    };
    drop(bots);
    
    bot.network.disconnect();
    
    Json(ApiResponse {
        success: true,
        data: Some(()),
        error: None,
    })
}

async fn collect_items(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let uuid = match Uuid::parse_str(&id) {
        Ok(u) => u,
        Err(_) => {
            return (StatusCode::BAD_REQUEST, Json(ApiResponse::<serde_json::Value> {
                success: false,
                data: None,
                error: Some("Invalid bot ID".to_string()),
            }));
        }
    };
    
    let bots = state.bots.read().unwrap();
    let bot = match bots.get(&uuid) {
        Some((b, _)) => b.clone(),
        None => {
            return (StatusCode::NOT_FOUND, Json(ApiResponse::<serde_json::Value> {
                success: false,
                data: None,
                error: Some("Bot not found".to_string()),
            }));
        }
    };
    drop(bots);
    
    let collected = bot.collect();
    
    (StatusCode::OK, Json(ApiResponse {
        success: true,
        data: Some(serde_json::json!({ "collected": collected })),
        error: None,
    }))
}

async fn leave_world(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let uuid = match Uuid::parse_str(&id) {
        Ok(u) => u,
        Err(_) => {
            return Json(ApiResponse::<()> {
                success: false,
                data: None,
                error: Some("Invalid bot ID".to_string()),
            });
        }
    };
    
    let bots = state.bots.read().unwrap();
    let bot = match bots.get(&uuid) {
        Some((b, _)) => b.clone(),
        None => {
            return Json(ApiResponse::<()> {
                success: false,
                data: None,
                error: Some("Bot not found".to_string()),
            });
        }
    };
    drop(bots);
    
    bot.leave();
    
    Json(ApiResponse {
        success: true,
        data: Some(()),
        error: None,
    })
}
