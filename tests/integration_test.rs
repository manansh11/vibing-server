use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::thread;
use std::time::Duration;

// A simple manual test to verify basic TCP connection handling
// Note: This is not testing the actual server implementation, but just TCP connectivity
#[test]
fn test_tcp_echo() {
    // Create a channel to notify when the server is ready
    let (tx, rx): (Sender<u16>, Receiver<u16>) = channel();
    
    // Spawn a thread for the echo server
    let server_thread = thread::spawn(move || {
        // Bind to a random port
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        
        // Notify the test that we're ready with the port number
        tx.send(port).unwrap();
        
        // Accept a connection
        let (mut stream, _) = listener.accept().unwrap();
        
        // Echo data back
        let mut buffer = [0; 1024];
        loop {
            match stream.read(&mut buffer) {
                Ok(0) => break, // Connection was closed
                Ok(n) => {
                    // Echo back what we read
                    stream.write_all(&buffer[0..n]).unwrap();
                }
                Err(_) => break,
            }
        }
    });
    
    // Wait for the server to be ready and get the port
    let port = rx.recv().unwrap();
    
    // Connect to the server
    let mut client = TcpStream::connect(format!("127.0.0.1:{}", port)).unwrap();
    
    // Set a read timeout
    client.set_read_timeout(Some(Duration::from_secs(1))).unwrap();
    
    // Send some data
    let test_data = b"Hello, Server!";
    client.write_all(test_data).unwrap();
    
    // Read the response
    let mut buffer = [0; 1024];
    let n = client.read(&mut buffer).unwrap();
    
    // Check that we got the same data back
    assert_eq!(&buffer[0..n], test_data);
    
    // Close the connection
    drop(client);
    
    // Wait for the server to finish
    server_thread.join().unwrap();
}

// A simple load test that sends multiple concurrent connections
#[test]
fn test_concurrent_connections() {
    const NUM_CLIENTS: usize = 10;
    const MESSAGES_PER_CLIENT: usize = 5;
    
    // Create a channel to notify when the server is ready
    let (tx, rx): (Sender<u16>, Receiver<u16>) = channel();
    
    // Spawn a thread for the echo server
    let server_thread = thread::spawn(move || {
        // Bind to a random port
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        
        // Notify the test that we're ready with the port number
        tx.send(port).unwrap();
        
        // Set the listener to non-blocking mode
        listener.set_nonblocking(true).unwrap();
        
        // Track active connections
        let mut connections = Vec::new();
        let mut connection_count = 0;
        
        // Process connections and data
        while connection_count < NUM_CLIENTS {
            // Try to accept a new connection
            match listener.accept() {
                Ok((stream, _)) => {
                    stream.set_nonblocking(true).unwrap();
                    connections.push(stream);
                    connection_count += 1;
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    // No new connections right now
                }
                Err(e) => panic!("Error accepting connection: {}", e),
            }
            
            // Process data on existing connections
            let mut i = 0;
            while i < connections.len() {
                let mut buffer = [0; 1024];
                match connections[i].read(&mut buffer) {
                    Ok(0) => {
                        // Connection closed
                        connections.remove(i);
                    }
                    Ok(n) => {
                        // Echo back what we read
                        if let Err(e) = connections[i].write_all(&buffer[0..n]) {
                            if e.kind() != std::io::ErrorKind::WouldBlock {
                                panic!("Error writing to connection: {}", e);
                            }
                        }
                        i += 1;
                    }
                    Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        // No data ready on this connection
                        i += 1;
                    }
                    Err(e) => {
                        panic!("Error reading from connection: {}", e);
                    }
                }
            }
            
            // A small delay to avoid busy-waiting
            thread::sleep(Duration::from_millis(1));
        }
        
        // Process remaining data until all connections are closed
        while !connections.is_empty() {
            let mut i = 0;
            while i < connections.len() {
                let mut buffer = [0; 1024];
                match connections[i].read(&mut buffer) {
                    Ok(0) => {
                        // Connection closed
                        connections.remove(i);
                    }
                    Ok(n) => {
                        // Echo back what we read
                        if let Err(e) = connections[i].write_all(&buffer[0..n]) {
                            if e.kind() != std::io::ErrorKind::WouldBlock {
                                panic!("Error writing to connection: {}", e);
                            }
                        }
                        i += 1;
                    }
                    Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        // No data ready on this connection
                        i += 1;
                    }
                    Err(e) => {
                        panic!("Error reading from connection: {}", e);
                    }
                }
            }
            
            // A small delay to avoid busy-waiting
            thread::sleep(Duration::from_millis(1));
        }
    });
    
    // Wait for the server to be ready and get the port
    let port = rx.recv().unwrap();
    
    // Spawn client threads
    let mut client_threads = Vec::with_capacity(NUM_CLIENTS);
    
    for client_id in 0..NUM_CLIENTS {
        let client_thread = thread::spawn(move || {
            // Connect to the server
            let mut client = TcpStream::connect(format!("127.0.0.1:{}", port)).unwrap();
            
            // Set a read timeout
            client.set_read_timeout(Some(Duration::from_secs(1))).unwrap();
            
            // Send and receive multiple messages
            for msg_id in 0..MESSAGES_PER_CLIENT {
                let test_data = format!("Message {} from client {}", msg_id, client_id).into_bytes();
                
                // Send the data
                client.write_all(&test_data).unwrap();
                
                // Read the response
                let mut buffer = [0; 1024];
                let n = client.read(&mut buffer).unwrap();
                
                // Check that we got the same data back
                assert_eq!(&buffer[0..n], &test_data);
            }
            
            // Close the connection
            drop(client);
        });
        
        client_threads.push(client_thread);
    }
    
    // Wait for all client threads to finish
    for thread in client_threads {
        thread.join().unwrap();
    }
    
    // Wait for the server to finish
    server_thread.join().unwrap();
}