pub mod acceptor;
pub mod buffer;
pub mod config;
pub mod connection;
pub mod error;
pub mod event_loop;
pub mod http;
pub mod memory;
pub mod metrics;
pub mod middleware;
pub mod router;
pub mod static_files;

/// Re-exports of common components for easier access
pub use acceptor::ConnectionAcceptor;
pub use config::ServerConfig;
pub use connection::Connection;
pub use error::{ServerError, ServerResult};
pub use event_loop::{EventLoop, EventPoller};
pub use http::{HttpParser, Method, Request, Response, Status};
pub use memory::{MemoryHandle, MemoryManager, MemoryPool};
pub use metrics::{Counter, Histogram, MetricsCollector, Timer};
pub use middleware::{
    MiddlewareChain, MiddlewareFn, MiddlewareNext,
    basic_auth_middleware, compression_middleware, content_type_middleware, 
    cors_middleware, logging_middleware,
};
pub use router::Router;
pub use static_files::{StaticFileConfig, add_static_file_routes, static_files_middleware};