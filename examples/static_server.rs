use high_performance_server::{
    ConnectionAcceptor, EventLoop, Method, MiddlewareChain, Request, Response, Router, ServerResult, Status,
    StaticFileConfig, add_static_file_routes, static_files_middleware,
    compression_middleware, logging_middleware,
};
use std::io;
use std::path::PathBuf;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

fn main() -> io::Result<()> {
    // Parse command-line arguments
    let args: Vec<String> = std::env::args().collect();
    let address = args.get(1).map(|s| s.as_str()).unwrap_or("127.0.0.1:8080");
    let static_dir = args.get(2).map(|s| s.to_string()).unwrap_or("static".to_string());
    
    println!("Starting static file server on {}", address);
    println!("Serving files from directory: {}", static_dir);
    
    // Create a router for API endpoints
    let mut router = Router::new();
    
    // Add some API endpoints
    router.get("/api/status", |_| {
        let info = serde_json::json!({
            "status": "running",
            "version": env!("CARGO_PKG_VERSION"),
            "server_time": chrono::Local::now().to_rfc3339(),
        });
        
        let mut response = Response::new(Status::Ok);
        response.set_header("Content-Type", "application/json");
        response.set_body(serde_json::to_string_pretty(&info).unwrap().as_bytes());
        Ok(response)
    });
    
    // Configure static file serving
    let static_file_config = StaticFileConfig {
        root_dir: PathBuf::from(static_dir),
        path_prefix: "/".to_string(),            // Serve files from the root URL
        index_file: "index.html".to_string(),
        follow_symlinks: false,
        directory_listing: true,                 // Enable directory listings
        max_file_size: 10 * 1024 * 1024,         // 10 MB
        cache_control: "public, max-age=3600".to_string(),
    };
    
    // Add static file routes to the router
    add_static_file_routes(&mut router, static_file_config.clone());
    
    // Create a shared router
    let router = Arc::new(router);
    
    // Create a middleware chain
    let mut middleware = MiddlewareChain::new();
    middleware.add(logging_middleware);
    middleware.add(compression_middleware);
    
    // Set the router as the final handler
    let router_clone = router.clone();
    middleware.set_handler(move |req| router_clone.handle_request(req));
    
    // Create a shared middleware chain
    let middleware = Arc::new(middleware);
    
    // Create a connection acceptor
    let acceptor = ConnectionAcceptor::new(address)?;
    let acceptor = Arc::new(acceptor);
    
    // Determine the number of CPU cores
    let cpu_cores = num_cpus::get();
    
    println!("Starting server with {} worker threads", cpu_cores);
    println!("Available routes:");
    println!("  GET  /               - Static file server");
    println!("  GET  /api/status     - Server status API");
    println!();
    println!("To test the server, open a browser at http://localhost:8080/");
    
    // Spawn one event loop per CPU core
    let mut handles = Vec::with_capacity(cpu_cores);
    
    for id in 0..cpu_cores {
        let acceptor_clone = acceptor.clone();
        let middleware_clone = middleware.clone();
        
        let handle = thread::spawn(move || {
            let mut event_loop = EventLoop::new(id as u32, acceptor_clone);
            
            // Set middleware chain for handling requests
            event_loop.set_middleware_chain(middleware_clone);
            
            // Run the event loop
            if let Err(e) = event_loop.run() {
                eprintln!("Event loop {} error: {:?}", id, e);
            }
        });
        
        handles.push(handle);
    }
    
    // Set up signal handling for graceful shutdown
    ctrlc::set_handler(move || {
        println!("Received shutdown signal. Stopping server...");
        std::process::exit(0);
    })
    .expect("Error setting Ctrl-C handler");
    
    // Wait for all threads to finish
    for handle in handles {
        let _ = handle.join();
    }
    
    Ok(())
}