use high_performance_server::{
    ConnectionAcceptor, EventLoop, Method, MiddlewareChain, Request, Response, Router, ServerResult, Status,
    compression_middleware, content_type_middleware, cors_middleware, logging_middleware,
};
use std::io;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

/// Main function - sets up the server with routing and middleware
fn main() -> io::Result<()> {
    // Parse command-line arguments
    let args: Vec<String> = std::env::args().collect();
    let address = args.get(1).map(|s| s.as_str()).unwrap_or("127.0.0.1:8080");
    
    println!("Starting high-performance web server on {}", address);
    
    // Create a router and set up routes
    let mut router = Router::new();
    
    // Home page
    router.get("/", |_| {
        let mut response = Response::new(Status::Ok);
        response.set_header("Content-Type", "text/html");
        response.set_body(include_bytes!("../static/index.html"));
        Ok(response)
    });
    
    // API route - get server info
    router.get("/api/info", |_| {
        let info = serde_json::json!({
            "name": "High-Performance Rust Server",
            "version": env!("CARGO_PKG_VERSION"),
            "uptime": "Just started",
            "requests_handled": 1,
            "cpu_cores": num_cpus::get(),
        });
        
        let mut response = Response::new(Status::Ok);
        response.set_header("Content-Type", "application/json");
        response.set_body(serde_json::to_string_pretty(&info).unwrap().as_bytes());
        Ok(response)
    });
    
    // Echo endpoint
    router.post("/api/echo", |req| {
        let mut response = Response::new(Status::Ok);
        
        // Set content type based on request
        if let Some(content_type) = req.get_header("content-type") {
            response.set_header("Content-Type", content_type);
        } else {
            response.set_header("Content-Type", "application/octet-stream");
        }
        
        // Echo back the request body
        response.set_body(&req.body);
        Ok(response)
    });
    
    // Hello route with path parameters
    let router_clone_for_hello = router.clone();
    router.get("/hello/:name", move |req| {
        let params = router_clone_for_hello.extract_params("/hello/:name", &req.uri);
        let binding = "World".to_string(); // Create a longer-lived value
        let name = params.get("name").unwrap_or(&binding);
        
        let mut response = Response::new(Status::Ok);
        response.set_header("Content-Type", "text/html");
        response.set_body(format!("<h1>Hello, {}!</h1>", name).as_bytes());
        Ok(response)
    });
    
    // Create a shared router
    let router = Arc::new(router);
    
    // Create a middleware chain
    let mut middleware = MiddlewareChain::new();
    middleware.add(logging_middleware);
    middleware.add(cors_middleware(vec!["*".to_string()]));
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