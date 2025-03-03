use high_performance_server::metrics::MetricsCollector;
use std::io::{self, Write};
use std::thread;
use std::time::Duration;
use std::sync::Arc;
use std::sync::Mutex;

fn main() -> io::Result<()> {
    println!("High-Performance Server Metrics Viewer");
    println!("=====================================");
    println!("Simulating server activity...");
    
    // Create a metrics collector wrapped in Arc to share between threads
    let collector = Arc::new(Mutex::new(MetricsCollector::new()));
    
    // Clone the Arc for the simulation thread
    let simulation_collector = collector.clone();
    
    // Spawn a thread to simulate server activity
    let simulation_thread = thread::spawn(move || {
        let mut connections = 0;
        let mut requests = 0;
        
        for _ in 0..60 {
            // Simulate new connections
            let new_connections = (rand::random::<u8>() % 5) as usize;
            for _ in 0..new_connections {
                simulation_collector.lock().unwrap().record_connection("accepted");
                connections += 1;
            }
            
            // Simulate requests on existing connections
            let new_requests = connections.min((rand::random::<u8>() % 10) as usize);
            for _ in 0..new_requests {
                // Simulate different HTTP methods and status codes
                let method = match rand::random::<u8>() % 3 {
                    0 => "GET",
                    1 => "POST",
                    _ => "PUT",
                };
                
                let status = match rand::random::<u8>() % 10 {
                    0 => 404,
                    1 => 500,
                    _ => 200,
                };
                
                // Record the request
                simulation_collector.lock().unwrap().record_request(method, status);
                requests += 1;
                
                // Simulate request timing
                {
                    let timer = simulation_collector.lock().unwrap().time_request(method);
                    // Simulate processing time
                    thread::sleep(Duration::from_millis(rand::random::<u8>() as u64 % 50));
                }
                
                // Simulate bytes transferred
                let bytes_received = (rand::random::<u16>() % 1000) as usize;
                let bytes_sent = (rand::random::<u16>() % 5000) as usize;
                simulation_collector.lock().unwrap().record_bytes_received(bytes_received);
                simulation_collector.lock().unwrap().record_bytes_sent(bytes_sent);
            }
            
            // Simulate closed connections
            let closed_connections = connections.min((rand::random::<u8>() % 3) as usize);
            for _ in 0..closed_connections {
                simulation_collector.lock().unwrap().record_connection("closed");
                connections -= 1;
            }
            
            // Wait for a bit
            thread::sleep(Duration::from_millis(500));
        }
    });
    
    // No need for this variable if we don't use it
    // let mut last_stats = String::new();
    
    for _ in 0..30 {
        // Clear screen (ANSI escape code)
        print!("\x1B[2J\x1B[1;1H");
        
        // Display current time
        let now = chrono::Local::now();
        println!("Time: {}", now.format("%Y-%m-%d %H:%M:%S"));
        println!("=====================================");
        
        // Get current metrics
        let stats = collector.lock().unwrap().format();
        println!("{}", stats);
        
        // Check if simulation thread is still running
        if simulation_thread.is_finished() {
            println!("Simulation complete.");
            break;
        }
        
        // Wait before updating again
        thread::sleep(Duration::from_secs(2));
    }
    
    // Wait for simulation to complete
    if !simulation_thread.is_finished() {
        println!("Waiting for simulation to complete...");
        simulation_thread.join().unwrap();
    }
    
    println!("Final metrics:");
    println!("=====================================");
    println!("{}", collector.lock().unwrap().format());
    
    Ok(())
}