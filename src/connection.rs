use crate::buffer::Buffer;
use std::io::{self, Write};
use std::net::{SocketAddr, TcpStream};
use std::time::{Duration, Instant};

/// Represents the current state of a connection
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionState {
    New,
    Reading,
    Processing,
    Writing,
    Closing,
    Closed,
}

/// Represents a TCP connection with a client
pub struct Connection {
    stream: TcpStream,
    peer_addr: SocketAddr,
    id: usize,
    state: ConnectionState,
    buffer: Buffer,
    last_activity: Instant,
    timeout: Duration,
}

impl Connection {
    /// Create a new connection from a TcpStream
    pub fn new(stream: TcpStream, peer_addr: SocketAddr, id: usize) -> io::Result<Self> {
        // Set TCP_NODELAY to disable Nagle's algorithm
        stream.set_nodelay(true)?;
        
        Ok(Self {
            stream,
            peer_addr,
            id,
            state: ConnectionState::New,
            buffer: Buffer::new(16 * 1024), // 16KB initial buffer
            last_activity: Instant::now(),
            timeout: Duration::from_secs(30), // 30 second default timeout
        })
    }
    
    /// Read data from the connection into the buffer
    pub fn read(&mut self) -> io::Result<usize> {
        self.state = ConnectionState::Reading;
        let bytes_read = self.buffer.read_from(&mut self.stream)?;
        self.last_activity = Instant::now();
        
        if bytes_read == 0 {
            // Remote end closed the connection
            self.state = ConnectionState::Closing;
        }
        
        Ok(bytes_read)
    }
    
    /// Write data to the connection
    pub fn write(&mut self, data: &[u8]) -> io::Result<usize> {
        self.state = ConnectionState::Writing;
        let result = self.stream.write(data);
        self.last_activity = Instant::now();
        result
    }
    
    /// Close the connection
    pub fn close(&mut self) -> io::Result<()> {
        self.state = ConnectionState::Closed;
        self.stream.shutdown(std::net::Shutdown::Both)
    }
    
    /// Check if the connection has timed out
    pub fn is_timed_out(&self) -> bool {
        self.last_activity.elapsed() > self.timeout
    }
    
    /// Get the connection's peer address
    pub fn peer_addr(&self) -> SocketAddr {
        self.peer_addr
    }
    
    /// Get the connection's unique ID
    pub fn id(&self) -> usize {
        self.id
    }
    
    /// Get a reference to the connection's buffer
    pub fn buffer(&self) -> &Buffer {
        &self.buffer
    }
    
    /// Get a mutable reference to the connection's buffer
    pub fn buffer_mut(&mut self) -> &mut Buffer {
        &mut self.buffer
    }
    
    /// Get the current state of the connection
    pub fn state(&self) -> ConnectionState {
        self.state
    }
    
    /// Set the state of the connection
    pub fn set_state(&mut self, state: ConnectionState) {
        self.state = state;
    }
    
    /// Set the connection timeout
    pub fn set_timeout(&mut self, timeout: Duration) {
        self.timeout = timeout;
    }
    
    /// Get a reference to the underlying TcpStream
    pub fn stream(&self) -> &TcpStream {
        &self.stream
    }
    
    /// Get a mutable reference to the underlying TcpStream
    pub fn stream_mut(&mut self) -> &mut TcpStream {
        &mut self.stream
    }
}