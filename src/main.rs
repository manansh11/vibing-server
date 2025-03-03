use high_performance_server::{ConnectionAcceptor, EventLoop, MetricsCollector, ServerConfig, ServerResult};
use std::io;
use std::num::NonZeroUsize;
use std::sync::Arc;
use std::path::Path;
use std::env;
use std::fs;
use std::time::Duration;

fn main() -> ServerResult<()> {
    // Parse command-line arguments
    let args: Vec<String> = env::args().collect();
    let config = if args.len() > 1 && Path::new(&args[1]).exists() {
        // Load configuration from file
        ServerConfig::from_json_file(&args[1])?
    } else {
        // Use default configuration
        ServerConfig::new()
    };
    
    // Create metrics collector
    let metrics = Arc::new(MetricsCollector::new());
    let metrics_clone = metrics.clone();
    
    // Create a connection acceptor that will bind to a specific address
    let address = config.socket_address();
    let acceptor = ConnectionAcceptor::new(&address)?;
    
    println!("Starting server on {} with {} worker threads", address, config.worker_threads);
    
    // Create a shared acceptor
    let acceptor = Arc::new(acceptor);
    
    // Start a metrics printer thread
    let metrics_thread = std::thread::spawn(move || {
        loop {
            // Sleep for a bit
            std::thread::sleep(Duration::from_secs(10));
            
            // Print current metrics
            println!("\n===== Server Metrics =====");
            println!("{}", metrics_clone.format());
            println!("==========================\n");
        }
    });
    
    // Spawn one event loop per worker thread
    let mut handles = Vec::with_capacity(config.worker_threads);
    
    for id in 0..config.worker_threads {
        let acceptor_clone = acceptor.clone();
        let handle = std::thread::spawn(move || {
            let mut event_loop = EventLoop::new(id as u32, acceptor_clone);
            event_loop.run()
        });
        handles.push(handle);
    }
    
    // Set up a signal handler for graceful shutdown
    ctrlc::set_handler(move || {
        println!("Received shutdown signal. Stopping server...");
        std::process::exit(0);
    }).expect("Error setting Ctrl-C handler");
    
    // Wait for all threads to complete (they shouldn't unless there's an error)
    for handle in handles {
        let _ = handle.join();
    }
    
    Ok(())
}

// Save default configuration to a file
fn save_default_config(path: &str) -> ServerResult<()> {
    let config = ServerConfig::new();
    config.save_to_json_file(path)?;
    println!("Default configuration saved to: {}", path);
    Ok(())
}