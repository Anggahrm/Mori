# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Mori is a cross-platform Growtopia companion tool written in Rust. It features a **web-based interface** for managing multiple game bots with support for both **official Growtopia servers** and **Private Servers (PS)**. The project uses a hybrid architecture with a core library (`gt-core`) that handles the game logic and networking, while the main application provides the web interface.

## Architecture

### Core Components

- **Main Application** (`src/main.rs`): Axum-based web server with REST API endpoints
- **Core Library** (`core/`): Contains all game logic, networking, and bot functionality
  - `Bot` struct: Main bot implementation with networking, automation, and scripting capabilities
  - Packet handling system for game protocol communication
  - A* pathfinding for bot movement
  - Lua scripting integration for automation
  - Inventory and world state management
  - **Private Server support**: Custom server configuration for PS connections

### Key Dependencies

- **Axum**: Web framework for REST API and web server
- **Tokio**: Async runtime for web server
- **tower-http**: HTTP middleware for CORS and static files
- **rusty_enet**: ENet networking library for game server communication
- **mlua**: Lua scripting integration for bot automation
- **serde_json**: JSON parsing for API responses
- **gtitem-r**: Item database parsing (external git dependency)
- **gtworld-r**: World data parsing (external git dependency)

## Development Commands

### Build and Run
```bash
cargo run                    # Run the web server (default port 3000)
cargo build                  # Build the project
cargo build --release        # Build optimized release version
PORT=8080 cargo run          # Run on custom port
```

### Core Library Development
```bash
cargo build -p gt-core       # Build only the core library
cargo test -p gt-core        # Run core library tests
```

## Project Structure

### Main Application (`src/`)
- `main.rs`: Axum web server with REST API endpoints for bot management
- `web/mod.rs`: Web utilities module (placeholder for future extensions)

### Core Library (`core/src/`)
- `lib.rs`: Main Bot struct and public API with Private Server support
- `types/`: Type definitions for game protocol and bot state
  - `server_config.rs`: Private Server configuration struct
- `packet_handler.rs`/`variant_handler.rs`: Network protocol handling
- `login.rs`: Authentication and login logic
- `server.rs`: Server communication with official and private server support
- `inventory.rs`: Inventory management
- `astar.rs`: A* pathfinding implementation
- `lua.rs`: Lua scripting engine integration
- `utils/`: Utility modules for protocol handling

### Templates (`templates/`)
- `index.html`: Web UI with Alpine.js and Tailwind CSS

## Web API Endpoints

### Bot Management
- `GET /api/bots` - List all bots
- `POST /api/bots` - Create new bot (supports private server config)
- `DELETE /api/bots/{id}` - Remove bot

### Bot Actions
- `POST /api/bots/{id}/connect` - Connect bot to server
- `POST /api/bots/{id}/disconnect` - Disconnect bot
- `POST /api/bots/{id}/warp` - Warp to world
- `POST /api/bots/{id}/say` - Send message
- `POST /api/bots/{id}/move` - Move bot (direction: up/down/left/right)
- `POST /api/bots/{id}/collect` - Collect nearby items
- `POST /api/bots/{id}/leave` - Leave current world

### Bot Information
- `GET /api/bots/{id}/inventory` - Get bot inventory
- `GET /api/bots/{id}/world` - Get world information
- `GET /api/bots/{id}/logs` - Get bot logs

## Login Methods

### Legacy Authentication
- Username/password credential input
- Uses internal token generation
- Recommended for Private Servers

### LTOKEN Authentication  
- Direct token input (4 colon-separated values)
- No token fetcher required
- Immediate bot creation upon validation

### Google/Apple Authentication
- Note: Browser automation not available in web mode
- Use Legacy or LTOKEN for Private Servers

## Private Server Support

The application supports connecting to Growtopia Private Servers:

### Configuration Options
- `server_ip`: Server IP address or hostname
- `server_port`: Server port (default: 17091)
- `server_data_url`: Optional custom URL for server_data.php
- `use_https`: Whether to use HTTPS for server data requests
- `skip_login_url`: Skip official login URL validation (default: true)

### Creating PS Bot via API
```json
POST /api/bots
{
  "login_method": "legacy",
  "credentials": {
    "growid": "username",
    "password": "password"
  },
  "private_server": {
    "server_ip": "192.168.1.100",
    "server_port": 17091,
    "use_https": false
  }
}
```

## Heroku Deployment

The application is configured for Heroku deployment:

### Files
- `Procfile`: Defines web process for Heroku
- Environment variable `PORT`: Automatically set by Heroku

### Deployment Steps
1. Create Heroku app with Rust buildpack
2. Push code to Heroku
3. Application starts on assigned port

### Buildpack
Use the official Rust buildpack:
```bash
heroku buildpacks:set emk/rust
```

## Key Features Implementation

- **Multi-bot Management**: Each bot runs in its own thread with Arc<Bot> for safe sharing
- **Real-time World State**: World data synchronized with game server
- **Automation System**: Configurable delays and automated actions
- **Scripting**: Embedded Lua for custom automation scripts
- **Item Database**: External item.dat file parsing for game items
- **Path Finding**: A* algorithm for intelligent bot movement
- **Private Server Support**: Full support for connecting to custom GT servers

## Important Notes

- Bot connections use ENet protocol with custom packet handling
- The project can optionally use `items.dat` file for item database functionality
- All bot operations are thread-safe using Mutex/RwLock patterns
- Network packets follow Growtopia's custom protocol implementation
- Each login method has distinct initialization flows
- Private Servers skip official authentication endpoints

## External Dependencies

The core library depends on custom Rust implementations of Growtopia protocols:
- `rusty_enet`: Custom ENet implementation
- `gtitem-r`: Item database parser
- `gtworld-r`: World data parser

These are maintained as separate Git repositories and may need updates when the game protocol changes.