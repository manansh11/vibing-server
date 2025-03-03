use crate::error::ServerResult;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use std::time::Duration;

/// Server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    // Network configuration
    pub listen_address: String,
    pub port: u16,
    pub backlog_size: u32,
    
    // Connection settings
    pub connection_timeout: Duration,
    pub initial_buffer_size: usize,
    
    // Thread configuration
    pub worker_threads: usize,
    
    // Memory configuration
    pub memory_pools_initial_size: usize,
    
    // HTTP configuration
    pub max_header_size: usize,
    pub max_request_size: usize,
    pub keep_alive: bool,
    pub keep_alive_timeout: Duration,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            listen_address: "127.0.0.1".to_string(),
            port: 8080,
            backlog_size: 1024,
            
            connection_timeout: Duration::from_secs(30),
            initial_buffer_size: 16 * 1024, // 16 KB
            
            worker_threads: num_cpus::get(),
            
            memory_pools_initial_size: 16,
            
            max_header_size: 16 * 1024, // 16 KB
            max_request_size: 1024 * 1024, // 1 MB
            keep_alive: true,
            keep_alive_timeout: Duration::from_secs(5),
        }
    }
}

impl ServerConfig {
    /// Create a new configuration with default values
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Set the address and port to listen on
    pub fn with_address(mut self, address: &str, port: u16) -> Self {
        self.listen_address = address.to_string();
        self.port = port;
        self
    }
    
    /// Set the connection timeout
    pub fn with_connection_timeout(mut self, timeout: Duration) -> Self {
        self.connection_timeout = timeout;
        self
    }
    
    /// Set the number of worker threads
    pub fn with_worker_threads(mut self, threads: usize) -> Self {
        self.worker_threads = threads;
        self
    }
    
    /// Set the initial buffer size for connections
    pub fn with_initial_buffer_size(mut self, size: usize) -> Self {
        self.initial_buffer_size = size;
        self
    }
    
    /// Get the full address string (address:port)
    pub fn socket_address(&self) -> String {
        format!("{}:{}", self.listen_address, self.port)
    }
    
    /// Load configuration from a JSON file
    pub fn from_json_file<P: AsRef<Path>>(path: P) -> ServerResult<Self> {
        let content = fs::read_to_string(path)?;
        let config = serde_json::from_str(&content)?;
        Ok(config)
    }
    
    /// Save configuration to a JSON file
    pub fn save_to_json_file<P: AsRef<Path>>(&self, path: P) -> ServerResult<()> {
        let content = serde_json::to_string_pretty(self)?;
        fs::write(path, content)?;
        Ok(())
    }
}