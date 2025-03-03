use crate::connection::Connection;
use socket2::{Domain, Protocol, Socket, Type};
use std::io;
use std::net::{SocketAddr, TcpListener, ToSocketAddrs};
use std::sync::atomic::{AtomicUsize, Ordering};

/// The ConnectionAcceptor is responsible for accepting new TCP connections
/// and distributing them across worker threads using a consistent hashing scheme.
pub struct ConnectionAcceptor {
    listener: TcpListener,
    address: String,
    connection_count: AtomicUsize,
    backlog_size: usize,
}

impl ConnectionAcceptor {
    /// Create a new connection acceptor bound to the specified address
    pub fn new<A: ToSocketAddrs>(addr: A) -> io::Result<Self> {
        // Convert the address to a string for later use
        let socket_addr = addr.to_socket_addrs()?.next().ok_or_else(|| {
            io::Error::new(io::ErrorKind::InvalidInput, "No socket addresses found")
        })?;
        let addr_str = socket_addr.to_string();
        
        // Create a socket with optimized settings
        let socket = Self::create_socket(&socket_addr)?;
        let listener = socket.into();
        
        Ok(Self {
            listener,
            address: addr_str,
            connection_count: AtomicUsize::new(0),
            backlog_size: 1024, // Default backlog size
        })
    }
    
    /// Accept a new connection
    pub fn accept(&self) -> io::Result<Connection> {
        let (stream, addr) = self.listener.accept()?;
        let count = self.connection_count.fetch_add(1, Ordering::Relaxed);
        
        // Configure the stream for non-blocking operation
        stream.set_nonblocking(true)?;
        
        // Create a new connection
        Connection::new(stream, addr, count)
    }
    
    /// Get the local address this acceptor is bound to
    pub fn local_addr(&self) -> io::Result<SocketAddr> {
        self.listener.local_addr()
    }
    
    /// Create a properly configured socket
    fn create_socket(addr: &SocketAddr) -> io::Result<Socket> {
        let domain = if addr.is_ipv6() {
            Domain::IPV6
        } else {
            Domain::IPV4
        };
        
        let socket = Socket::new(domain, Type::STREAM, Some(Protocol::TCP))?;
        
        // Set socket options for better performance
        socket.set_nonblocking(true)?;
        socket.set_reuse_address(true)?;
        
        #[cfg(unix)]
        socket.set_reuse_port(true)?;
        
        // Bind the socket - fixing for cross-platform compatibility
        let sock_addr = socket2::SockAddr::from(*addr);
        socket.bind(&sock_addr)?;
        
        // Start listening with a large backlog
        socket.listen(1024)?;
        
        Ok(socket)
    }
    
    /// Distribute a connection across event loops based on consistent hashing
    pub fn distribute_connection(&self, connection: Connection, thread_count: usize) -> usize {
        // Simple distribution strategy - round robin based on connection count
        // In a production system, this would use a more sophisticated consistent hashing approach
        self.connection_count.load(Ordering::Relaxed) % thread_count
    }
}