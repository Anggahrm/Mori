use clap::{Parser, ValueEnum};
use gt_core::types::bot::LoginVia;
use gt_core::types::login_info::PrivateServerConfig;
use gt_core::{Bot, EventType};
use std::fs::File;
use std::io::Read;
use std::sync::{Arc, RwLock};
use std::thread;

#[derive(Parser)]
#[command(name = "mori-cli")]
#[command(about = "Growtopia bot CLI for VPS deployment", long_about = None)]
struct Cli {
    /// Login method
    #[arg(short, long, value_enum, default_value = "legacy")]
    login_method: LoginMethod,

    /// Username (for legacy login)
    #[arg(short, long)]
    username: Option<String>,

    /// Password (for legacy login)
    #[arg(short, long)]
    password: Option<String>,

    /// LTOKEN value (format: value1:value2:value3:value4)
    #[arg(long)]
    ltoken: Option<String>,

    /// Path to items.dat file
    #[arg(short, long, default_value = "items.dat")]
    items_dat: String,

    /// Use private server
    #[arg(long)]
    private_server: bool,

    /// Private server host (e.g., www.growtopia1.com or custom domain)
    #[arg(long)]
    ps_host: Option<String>,

    /// Private server IP address
    #[arg(long)]
    ps_ip: Option<String>,

    /// Private server port (default: 17091)
    #[arg(long, default_value = "17091")]
    ps_port: u16,
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
enum LoginMethod {
    Legacy,
    Ltoken,
    Google,
    Apple,
}

fn main() {
    let cli = Cli::parse();

    println!("Mori CLI - Growtopia Bot");
    println!("========================");

    // Load items.dat
    let item_database = match load_items_dat(&cli.items_dat) {
        Ok(db) => {
            println!("[OK] Loaded items.dat with {} items", db.item_count);
            Arc::new(RwLock::new(db))
        }
        Err(e) => {
            eprintln!("[ERROR] Failed to load items.dat: {}", e);
            eprintln!("Please ensure items.dat is in the current directory or specify path with --items-dat");
            std::process::exit(1);
        }
    };

    // Setup login via
    let login_via = match cli.login_method {
        LoginMethod::Legacy => {
            let username = cli.username.unwrap_or_else(|| {
                eprintln!("[ERROR] Username required for legacy login (--username)");
                std::process::exit(1);
            });
            let password = cli.password.unwrap_or_else(|| {
                eprintln!("[ERROR] Password required for legacy login (--password)");
                std::process::exit(1);
            });
            println!("[INFO] Using legacy login for user: {}", username);
            LoginVia::LEGACY([username, password])
        }
        LoginMethod::Ltoken => {
            let ltoken = cli.ltoken.unwrap_or_else(|| {
                eprintln!("[ERROR] LTOKEN required (--ltoken value1:value2:value3:value4)");
                std::process::exit(1);
            });
            let parts: Vec<&str> = ltoken.split(':').collect();
            if parts.len() != 4 {
                eprintln!("[ERROR] LTOKEN must have 4 parts separated by colons");
                std::process::exit(1);
            }
            println!("[INFO] Using LTOKEN login");
            LoginVia::LTOKEN([
                parts[0].to_string(),
                parts[1].to_string(),
                parts[2].to_string(),
                parts[3].to_string(),
            ])
        }
        LoginMethod::Google => {
            println!("[INFO] Using Google login (requires token fetcher - not available in CLI mode)");
            eprintln!("[ERROR] Google login is not supported in CLI mode. Use LTOKEN or legacy login.");
            std::process::exit(1);
        }
        LoginMethod::Apple => {
            println!("[INFO] Using Apple login (requires token fetcher - not available in CLI mode)");
            eprintln!("[ERROR] Apple login is not supported in CLI mode. Use LTOKEN or legacy login.");
            std::process::exit(1);
        }
    };

    // Setup private server config
    let private_server_config = if cli.private_server {
        let host = cli.ps_host.unwrap_or_else(|| {
            eprintln!("[ERROR] Private server host required (--ps-host)");
            std::process::exit(1);
        });
        let ip = cli.ps_ip.unwrap_or_else(|| {
            eprintln!("[ERROR] Private server IP required (--ps-ip)");
            std::process::exit(1);
        });
        println!("[INFO] Using private server: {} ({}:{})", host, ip, cli.ps_port);
        Some(PrivateServerConfig::new(&host, &ip, cli.ps_port))
    } else {
        println!("[INFO] Using official Growtopia servers");
        None
    };

    // Create bot
    let (bot, event_rx) = Bot::new_with_private_server(
        login_via,
        None, // No token fetcher in CLI mode
        item_database,
        None, // No SOCKS5 proxy for now
        private_server_config,
    );

    println!("[INFO] Bot created, starting connection...");

    // Spawn event listener thread
    thread::spawn(move || {
        while let Ok(event) = event_rx.recv() {
            match &event.event_type {
                EventType::Connected { server, port } => {
                    println!("[EVENT] Connected to {}:{}", server, port);
                }
                EventType::Disconnected { reason } => {
                    println!("[EVENT] Disconnected: {:?}", reason);
                }
                EventType::PositionChanged { x, y } => {
                    println!("[EVENT] Position: ({}, {})", x, y);
                }
                EventType::Log { level, message } => {
                    println!("[LOG:{:?}] {}", level, message);
                }
                _ => {
                    println!("[EVENT] {:?}", event.event_type);
                }
            }
        }
    });

    // Start the bot
    bot.logon(None);

    println!("[INFO] Bot is running. Press Ctrl+C to stop.");

    // Keep main thread alive
    loop {
        thread::sleep(std::time::Duration::from_secs(1));
    }
}

fn load_items_dat(path: &str) -> Result<gt_core::gtitem_r::structs::ItemDatabase, String> {
    let mut file = File::open(path).map_err(|e| format!("Cannot open file: {}", e))?;
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)
        .map_err(|e| format!("Cannot read file: {}", e))?;
    
    gt_core::gtitem_r::load_from_memory(&buffer)
        .map_err(|e| format!("Cannot parse items.dat: {}", e))
}
