use std::io::{Read, Write};
use std::net::TcpStream;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

fn main() {
    // Configuration
    let server_address = "127.0.0.1:8080";
    let num_threads = 10;
    let requests_per_thread = 1000;
    let request_interval_ms = 10; // Milliseconds between requests
    
    println!("Load testing server at {}", server_address);
    println!("Threads: {}", num_threads);
    println!("Requests per thread: {}", requests_per_thread);
    println!("Total requests: {}", num_threads * requests_per_thread);
    println!("Request interval: {} ms", request_interval_ms);
    
    // Statistics
    let total_requests = Arc::new(AtomicUsize::new(0));
    let successful_requests = Arc::new(AtomicUsize::new(0));
    let failed_requests = Arc::new(AtomicUsize::new(0));
    let request_times = Arc::new(Mutex::new(Vec::with_capacity(num_threads * requests_per_thread)));
    
    // Start time
    let start_time = Instant::now();
    
    // Spawn worker threads
    let mut handles = Vec::with_capacity(num_threads);
    
    for thread_id in 0..num_threads {
        let total_requests = total_requests.clone();
        let successful_requests = successful_requests.clone();
        let failed_requests = failed_requests.clone();
        let request_times = request_times.clone();
        let server_address = server_address.to_string();
        
        let handle = thread::spawn(move || {
            for i in 0..requests_per_thread {
                let request_id = (thread_id * requests_per_thread) + i;
                
                // Make a request
                let request_start = Instant::now();
                match make_request(&server_address, request_id) {
                    Ok(response_size) => {
                        let request_time = request_start.elapsed();
                        successful_requests.fetch_add(1, Ordering::Relaxed);
                        
                        // Record the request time
                        let mut times = request_times.lock().unwrap();
                        times.push(request_time.as_micros() as f64 / 1000.0); // Convert to milliseconds
                    }
                    Err(e) => {
                        eprintln!("Request {} failed: {}", request_id, e);
                        failed_requests.fetch_add(1, Ordering::Relaxed);
                    }
                }
                
                total_requests.fetch_add(1, Ordering::Relaxed);
                
                // Sleep between requests to avoid overwhelming the server
                thread::sleep(Duration::from_millis(request_interval_ms));
            }
        });
        
        handles.push(handle);
    }
    
    // Status reporting thread
    let status_thread = {
        let total_requests = total_requests.clone();
        let successful_requests = successful_requests.clone();
        let failed_requests = failed_requests.clone();
        
        thread::spawn(move || {
            let total_expected = num_threads * requests_per_thread;
            
            while total_requests.load(Ordering::Relaxed) < total_expected {
                let completed = total_requests.load(Ordering::Relaxed);
                let successful = successful_requests.load(Ordering::Relaxed);
                let failed = failed_requests.load(Ordering::Relaxed);
                let elapsed = start_time.elapsed().as_secs_f64();
                let rps = completed as f64 / elapsed;
                
                println!(
                    "Progress: {}/{} ({:.1}%) - Success: {} Failed: {} - {:.1} req/sec",
                    completed,
                    total_expected,
                    (completed as f64 / total_expected as f64) * 100.0,
                    successful,
                    failed,
                    rps
                );
                
                thread::sleep(Duration::from_secs(1));
            }
        })
    };
    
    // Wait for all worker threads to complete
    for handle in handles {
        handle.join().unwrap();
    }
    
    // Total elapsed time
    let total_time = start_time.elapsed();
    
    // Calculate statistics
    let total_completed = total_requests.load(Ordering::Relaxed);
    let successful = successful_requests.load(Ordering::Relaxed);
    let failed = failed_requests.load(Ordering::Relaxed);
    let elapsed_secs = total_time.as_secs_f64();
    let requests_per_second = total_completed as f64 / elapsed_secs;
    
    // Calculate percentiles
    let mut times = request_times.lock().unwrap();
    times.sort_by(|a, b| a.partial_cmp(b).unwrap());
    
    let p50 = percentile(&times, 0.5);
    let p90 = percentile(&times, 0.9);
    let p95 = percentile(&times, 0.95);
    let p99 = percentile(&times, 0.99);
    
    // Print final statistics
    println!("\nLoad Test Results");
    println!("=================");
    println!("Total time: {:.2} seconds", elapsed_secs);
    println!("Total requests: {}", total_completed);
    println!("Successful requests: {}", successful);
    println!("Failed requests: {}", failed);
    println!("Requests per second: {:.2}", requests_per_second);
    println!("\nLatency (ms):");
    println!("  50th percentile: {:.2}", p50);
    println!("  90th percentile: {:.2}", p90);
    println!("  95th percentile: {:.2}", p95);
    println!("  99th percentile: {:.2}", p99);
    println!("  min: {:.2}", times.first().unwrap_or(&0.0));
    println!("  max: {:.2}", times.last().unwrap_or(&0.0));
    
    // Wait for status thread to finish
    status_thread.join().unwrap();
}

// Make a simple HTTP request to the server
fn make_request(server_address: &str, request_id: usize) -> Result<usize, Box<dyn std::error::Error>> {
    // Connect to the server
    let mut stream = TcpStream::connect(server_address)?;
    
    // Set timeouts
    stream.set_read_timeout(Some(Duration::from_secs(5)))?;
    stream.set_write_timeout(Some(Duration::from_secs(5)))?;
    
    // Create a simple HTTP request
    let request = format!(
        "GET /load-test/{} HTTP/1.1\r\n\
         Host: localhost\r\n\
         User-Agent: load-test-client\r\n\
         Connection: close\r\n\
         \r\n",
        request_id
    );
    
    // Send the request
    stream.write_all(request.as_bytes())?;
    
    // Read the response
    let mut response = Vec::new();
    stream.read_to_end(&mut response)?;
    
    Ok(response.len())
}

// Calculate percentile from a sorted array
fn percentile(sorted_data: &[f64], percentile: f64) -> f64 {
    if sorted_data.is_empty() {
        return 0.0;
    }
    
    let index = (sorted_data.len() as f64 * percentile).ceil() as usize - 1;
    sorted_data[index.min(sorted_data.len() - 1)]
}