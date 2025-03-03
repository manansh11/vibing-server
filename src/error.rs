use std::io;
use thiserror::Error;

/// Main error type for the server
#[derive(Error, Debug)]
pub enum ServerError {
    #[error("IO error: {0}")]
    Io(#[from] io::Error),
    
    #[error("HTTP parsing error: {0}")]
    HttpParse(String),
    
    #[error("Buffer error: {0}")]
    Buffer(String),
    
    #[error("Memory allocation error: {0}")]
    Memory(String),
    
    #[error("Connection error: {0}")]
    Connection(String),
    
    #[error("Event loop error: {0}")]
    EventLoop(String),
    
    #[error("Protocol error: {0}")]
    Protocol(String),
    
    #[error("Configuration error: {0}")]
    Config(String),
    
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}

pub type ServerResult<T> = Result<T, ServerError>;