use serde::{Deserialize, Serialize};

/// Configuration for connecting to a Private Server
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrivateServerConfig {
    /// Server IP address or hostname
    pub server_ip: String,
    /// Server port (typically 17091)
    pub server_port: u16,
    /// Login URL for server data endpoint (e.g., "www.yourps.com")
    pub server_data_url: String,
    /// Whether to use HTTPS for server data requests
    pub use_https: bool,
    /// Whether to skip login URL validation
    pub skip_login_url: bool,
}

impl PrivateServerConfig {
    pub fn new(server_ip: String, server_port: u16, server_data_url: String) -> Self {
        Self {
            server_ip,
            server_port,
            server_data_url,
            use_https: true,
            skip_login_url: true,
        }
    }

    /// Create config for a typical private server setup
    pub fn simple(host: &str, port: u16) -> Self {
        Self {
            server_ip: host.to_string(),
            server_port: port,
            server_data_url: host.to_string(),
            use_https: false,
            skip_login_url: true,
        }
    }

    /// Get the full server data URL
    pub fn get_server_data_url(&self) -> String {
        let protocol = if self.use_https { "https" } else { "http" };
        format!("{}://{}/growtopia/server_data.php", protocol, self.server_data_url)
    }
}

impl Default for PrivateServerConfig {
    fn default() -> Self {
        Self {
            server_ip: "127.0.0.1".to_string(),
            server_port: 17091,
            server_data_url: "localhost".to_string(),
            use_https: false,
            skip_login_url: true,
        }
    }
}
