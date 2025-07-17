use crate::acceptor::ConnectionAcceptor;
use crate::connection::{Connection, ConnectionState};
use crate::error::{ServerError, ServerResult};
use crate::http::{HttpParser, Request, Response, Status};
use std::collections::HashMap;
use std::io::{self, ErrorKind, Write};
use std::sync::Arc;
use std::time::Instant;

#[cfg(target_os = "linux")]
use libc::{EPOLLERR, EPOLLET, EPOLLIN, EPOLLOUT, EPOLLRDHUP};

#[cfg(target_os = "macos")]
use std::os::unix::io::AsRawFd;
#[cfg(target_os = "macos")]
use libc::{kqueue, kevent, timespec, EVFILT_READ, EVFILT_WRITE, EV_ADD, EV_DELETE, EV_EOF, EV_ERROR};

/// An abstraction for platform-specific event polling
#[cfg(target_os = "linux")]
pub struct EventPoller {
    epoll_fd: i32,
    events: Vec<libc::epoll_event>,
    max_events: usize,
}

#[cfg(target_os = "macos")]
pub struct EventPoller {
    kqueue_fd: i32,
    events: Vec<libc::kevent>,
    max_events: usize,
    // Map to track connection IDs to file descriptors
    conn_map: HashMap<usize, i32>,
}

#[cfg(target_os = "windows")]
pub struct EventPoller {
    // Windows implementation would use IOCP
    iocp_handle: usize,
    max_events: usize,
}

#[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
pub struct EventPoller {
    max_events: usize,
}

// Linux implementation
#[cfg(target_os = "linux")]
impl EventPoller {
    /// Create a new event poller
    pub fn new(max_events: usize) -> ServerResult<Self> {
        let epoll_fd = unsafe { libc::epoll_create1(0) };
        if epoll_fd < 0 {
            return Err(ServerError::Io(io::Error::last_os_error()));
        }
        
        let events = Vec::with_capacity(max_events);
        
        Ok(Self {
            epoll_fd,
            events,
            max_events,
        })
    }
    
    /// Register a connection with the poller
    pub fn register(&mut self, connection: &Connection) -> ServerResult<()> {
        let fd = connection.stream().as_raw_fd();
        let mut event = libc::epoll_event {
            events: (EPOLLIN | EPOLLOUT | EPOLLET | EPOLLRDHUP) as u32,
            u64: connection.id() as u64,
        };
        
        let ret = unsafe {
            libc::epoll_ctl(
                self.epoll_fd,
                libc::EPOLL_CTL_ADD,
                fd,
                &mut event as *mut _,
            )
        };
        
        if ret < 0 {
            return Err(ServerError::Io(io::Error::last_os_error()));
        }
        
        Ok(())
    }
    
    /// Deregister a connection from the poller
    pub fn deregister(&mut self, connection: &Connection) -> ServerResult<()> {
        let fd = connection.stream().as_raw_fd();
        let ret = unsafe {
            libc::epoll_ctl(
                self.epoll_fd,
                libc::EPOLL_CTL_DEL,
                fd,
                std::ptr::null_mut(),
            )
        };
        
        if ret < 0 {
            return Err(ServerError::Io(io::Error::last_os_error()));
        }
        
        Ok(())
    }
    
    /// Poll for events with a timeout
    pub fn poll(&mut self, timeout_ms: i32) -> ServerResult<Vec<(usize, u32)>> {
        self.events.clear();
        self.events.resize(self.max_events, libc::epoll_event { events: 0, u64: 0 });
        
        let num_events = unsafe {
            libc::epoll_wait(
                self.epoll_fd,
                self.events.as_mut_ptr(),
                self.max_events as i32,
                timeout_ms,
            )
        };
        
        if num_events < 0 {
            let err = io::Error::last_os_error();
            // Ignore EINTR as it's just a signal interruption
            if err.kind() != ErrorKind::Interrupted {
                return Err(ServerError::Io(err));
            }
            return Ok(Vec::new());
        }
        
        let result = self.events[..num_events as usize]
            .iter()
            .map(|event| (event.u64 as usize, event.events))
            .collect();
        
        Ok(result)
    }
}

// macOS implementation
#[cfg(target_os = "macos")]
impl EventPoller {
    /// Create a new event poller using kqueue (macOS)
    pub fn new(max_events: usize) -> ServerResult<Self> {
        let kqueue_fd = unsafe { kqueue() };
        if kqueue_fd < 0 {
            return Err(ServerError::Io(io::Error::last_os_error()));
        }
        
        let events = Vec::with_capacity(max_events);
        
        Ok(Self {
            kqueue_fd,
            events,
            max_events,
            conn_map: HashMap::new(),
        })
    }
    
    /// Register a connection with the poller
    pub fn register(&mut self, connection: &Connection) -> ServerResult<()> {
        let fd = connection.stream().as_raw_fd();
        let conn_id = connection.id();
        
        // Set up read event
        let read_event = libc::kevent {
            ident: fd as usize,
            filter: EVFILT_READ as i16,
            flags: EV_ADD as u16,
            fflags: 0,
            data: 0,
            udata: conn_id as *mut libc::c_void,
        };
        
        // Set up write event
        let write_event = libc::kevent {
            ident: fd as usize,
            filter: EVFILT_WRITE as i16,
            flags: EV_ADD as u16,
            fflags: 0,
            data: 0,
            udata: conn_id as *mut libc::c_void,
        };
        
        let changelist = [read_event, write_event];
        
        let ret = unsafe {
            kevent(
                self.kqueue_fd,
                changelist.as_ptr(),
                2, // Two events in changelist
                std::ptr::null_mut(),
                0,
                std::ptr::null(),
            )
        };
        
        if ret < 0 {
            return Err(ServerError::Io(io::Error::last_os_error()));
        }
        
        // Store connection ID to fd mapping
        self.conn_map.insert(conn_id, fd);
        
        Ok(())
    }
    
    /// Deregister a connection from the poller
    pub fn deregister(&mut self, connection: &Connection) -> ServerResult<()> {
        let fd = connection.stream().as_raw_fd();
        let conn_id = connection.id();
        
        // Set up read event deletion
        let read_event = libc::kevent {
            ident: fd as usize,
            filter: EVFILT_READ as i16,
            flags: EV_DELETE as u16,
            fflags: 0,
            data: 0,
            udata: conn_id as *mut libc::c_void,
        };
        
        // Set up write event deletion
        let write_event = libc::kevent {
            ident: fd as usize,
            filter: EVFILT_WRITE as i16,
            flags: EV_DELETE as u16,
            fflags: 0,
            data: 0,
            udata: conn_id as *mut libc::c_void,
        };
        
        let changelist = [read_event, write_event];
        
        let ret = unsafe {
            kevent(
                self.kqueue_fd,
                changelist.as_ptr(),
                2, // Two events in changelist
                std::ptr::null_mut(),
                0,
                std::ptr::null(),
            )
        };
        
        if ret < 0 {
            // Ignore errors when removing non-existent events
            let err = io::Error::last_os_error();
            if err.kind() != ErrorKind::NotFound {
                return Err(ServerError::Io(err));
            }
        }
        
        // Remove connection ID from mapping
        self.conn_map.remove(&conn_id);
        
        Ok(())
    }
    
    /// Poll for events with a timeout
    pub fn poll(&mut self, timeout_ms: i32) -> ServerResult<Vec<(usize, u32)>> {
        self.events.clear();
        self.events.resize(self.max_events, unsafe { std::mem::zeroed() });
        
        // Set up timeout
        let timeout = timespec {
            tv_sec: (timeout_ms / 1000) as i64,
            tv_nsec: ((timeout_ms % 1000) * 1_000_000) as i64,
        };
        
        let num_events = unsafe {
            kevent(
                self.kqueue_fd,
                std::ptr::null(),
                0,
                self.events.as_mut_ptr(),
                self.max_events as i32,
                &timeout,
            )
        };
        
        if num_events < 0 {
            let err = io::Error::last_os_error();
            // Ignore EINTR as it's just a signal interruption
            if err.kind() != ErrorKind::Interrupted {
                return Err(ServerError::Io(err));
            }
            return Ok(Vec::new());
        }
        
        let mut result = Vec::with_capacity(num_events as usize);
        
        for i in 0..num_events as usize {
            let event = &self.events[i];
            
            // Get connection ID from udata
            let conn_id = event.udata as usize;
            
            // Convert kqueue events to our internal event format (similar to epoll)
            let mut flags: u32 = 0;
            
            if event.filter == EVFILT_READ as i16 {
                flags |= 0x001; // Similar to EPOLLIN
            }
            
            if event.filter == EVFILT_WRITE as i16 {
                flags |= 0x004; // Similar to EPOLLOUT
            }
            
            if (event.flags & EV_EOF as u16) != 0 {
                flags |= 0x008; // Similar to EPOLLRDHUP
            }
            
            if (event.flags & EV_ERROR as u16) != 0 {
                flags |= 0x008 | 0x010; // Similar to EPOLLRDHUP | EPOLLERR
            }
            
            result.push((conn_id, flags));
        }
        
        Ok(result)
    }
}

// Windows implementation (stub)
#[cfg(target_os = "windows")]
impl EventPoller {
    pub fn new(_max_events: usize) -> ServerResult<Self> {
        // Windows implementation would use IOCP
        unimplemented!("Windows support not yet implemented");
    }
    
    pub fn register(&mut self, _connection: &Connection) -> ServerResult<()> {
        unimplemented!("Windows support not yet implemented");
    }
    
    pub fn deregister(&mut self, _connection: &Connection) -> ServerResult<()> {
        unimplemented!("Windows support not yet implemented");
    }
    
    pub fn poll(&mut self, _timeout_ms: i32) -> ServerResult<Vec<(usize, u32)>> {
        unimplemented!("Windows support not yet implemented");
    }
}

// Fallback implementation for other platforms (stubs)
#[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
impl EventPoller {
    pub fn new(max_events: usize) -> ServerResult<Self> {
        Err(ServerError::EventLoop("Unsupported platform".to_string()))
    }
    
    pub fn register(&mut self, _connection: &Connection) -> ServerResult<()> {
        Err(ServerError::EventLoop("Unsupported platform".to_string()))
    }
    
    pub fn deregister(&mut self, _connection: &Connection) -> ServerResult<()> {
        Err(ServerError::EventLoop("Unsupported platform".to_string()))
    }
    
    pub fn poll(&mut self, _timeout_ms: i32) -> ServerResult<Vec<(usize, u32)>> {
        Err(ServerError::EventLoop("Unsupported platform".to_string()))
    }
}

impl Drop for EventPoller {
    fn drop(&mut self) {
        #[cfg(target_os = "linux")]
        unsafe {
            libc::close(self.epoll_fd);
        }
        
        #[cfg(target_os = "macos")]
        unsafe {
            libc::close(self.kqueue_fd);
        }
    }
}

/// The main event loop for handling connections
pub struct EventLoop {
    thread_id: u32,
    poller: EventPoller,
    connections: HashMap<usize, Connection>,
    acceptor: Arc<ConnectionAcceptor>,
    parsers: HashMap<usize, HttpParser>,
    running: bool,
    router: Option<Arc<crate::router::Router>>,
    middleware_chain: Option<Arc<crate::middleware::MiddlewareChain>>,
}

impl EventLoop {
    /// Create a new event loop
    pub fn new(thread_id: u32, acceptor: Arc<ConnectionAcceptor>) -> Self {
        let poller = EventPoller::new(1024).expect("Failed to create event poller");
        
        Self {
            thread_id,
            poller,
            connections: HashMap::new(),
            acceptor,
            parsers: HashMap::new(),
            running: false,
            router: None,
            middleware_chain: None,
        }
    }
    
    /// Run the event loop
    pub fn run(&mut self) -> ServerResult<()> {
        self.running = true;
        
        while self.running {
            // Accept new connections
            self.accept_connections()?;
            
            // Poll for events
            let events = self.poller.poll(100)?;
            
            // Process events
            for (conn_id, event_bits) in events {
                self.process_connection_event(conn_id, event_bits)?;
            }
            
            // Check for timed out connections
            self.check_timeouts()?;
        }
        
        Ok(())
    }
    
    /// Stop the event loop
    pub fn stop(&mut self) {
        self.running = false;
    }
    
    /// Set the router for handling requests
    pub fn set_router(&mut self, router: Arc<crate::router::Router>) {
        self.router = Some(router);
    }
    
    /// Set the middleware chain for handling requests
    pub fn set_middleware_chain(&mut self, middleware_chain: Arc<crate::middleware::MiddlewareChain>) {
        self.middleware_chain = Some(middleware_chain);
    }
    
    /// Accept new connections
    fn accept_connections(&mut self) -> ServerResult<()> {
        // Try to accept multiple connections in a batch
        for _ in 0..10 {
            match self.acceptor.accept() {
                Ok(conn) => {
                    let conn_id = conn.id();
                    
                    // Register with the poller
                    self.poller.register(&conn)?;
                    
                    // Create a parser for this connection
                    let parser = HttpParser::new();
                    
                    // Store the connection and parser
                    self.connections.insert(conn_id, conn);
                    self.parsers.insert(conn_id, parser);
                }
                Err(ref e) if e.kind() == ErrorKind::WouldBlock => {
                    // No more connections to accept right now
                    break;
                }
                Err(e) => {
                    return Err(ServerError::Io(e));
                }
            }
        }
        
        Ok(())
    }
    
    /// Process an event for a connection
    fn process_connection_event(&mut self, conn_id: usize, event_bits: u32) -> ServerResult<()> {
        // Define constants for our platform-agnostic event types
        const EVENT_READ: u32 = 0x001;  // EPOLLIN equivalent
        const EVENT_WRITE: u32 = 0x004; // EPOLLOUT equivalent
        const EVENT_HUP: u32 = 0x008;   // EPOLLRDHUP equivalent
        const EVENT_ERR: u32 = 0x010;   // EPOLLERR equivalent
        
        #[cfg(target_os = "linux")]
        {
            let readable = (event_bits & EPOLLIN as u32) != 0;
            let writable = (event_bits & EPOLLOUT as u32) != 0;
            let error = (event_bits & (EPOLLERR | EPOLLRDHUP) as u32) != 0;
            
            // Handle error condition
            if error {
                self.close_connection(conn_id)?;
                return Ok(());
            }
            
            // Handle readable event
            if readable {
                self.handle_read(conn_id)?;
            }
            
            // Handle writable event
            if writable {
                self.handle_write(conn_id)?;
            }
        }
        
        #[cfg(target_os = "macos")]
        {
            let readable = (event_bits & EVENT_READ) != 0;
            let writable = (event_bits & EVENT_WRITE) != 0;
            let error = (event_bits & (EVENT_HUP | EVENT_ERR)) != 0;
            
            // Handle error condition
            if error {
                self.close_connection(conn_id)?;
                return Ok(());
            }
            
            // Handle readable event
            if readable {
                self.handle_read(conn_id)?;
            }
            
            // Handle writable event
            if writable {
                self.handle_write(conn_id)?;
            }
        }
        
        #[cfg(not(any(target_os = "linux", target_os = "macos")))]
        {
            // For other platforms, we'll implement a minimal version
            // that just forwards to our read/write handlers based on event bits
            let readable = (event_bits & EVENT_READ) != 0;
            let writable = (event_bits & EVENT_WRITE) != 0;
            let error = (event_bits & (EVENT_HUP | EVENT_ERR)) != 0;
            
            // Handle error condition
            if error {
                self.close_connection(conn_id)?;
                return Ok(());
            }
            
            // Handle readable event
            if readable {
                self.handle_read(conn_id)?;
            }
            
            // Handle writable event
            if writable {
                self.handle_write(conn_id)?;
            }
        }
        
        Ok(())
    }
    
    /// Handle a read event
    fn handle_read(&mut self, conn_id: usize) -> ServerResult<()> {
        let connection = match self.connections.get_mut(&conn_id) {
            Some(conn) => conn,
            None => return Ok(()),
        };
        
        // Read data from the connection
        match connection.read() {
            Ok(0) => {
                // Connection closed by peer
                self.close_connection(conn_id)?;
                return Ok(());
            }
            Ok(_) => {
                // Process the received data
                self.process_data(conn_id)?;
            }
            Err(ref e) if e.kind() == ErrorKind::WouldBlock => {
                // Nothing to read right now
            }
            Err(e) => {
                // Error reading
                println!("Error reading from connection {}: {}", conn_id, e);
                self.close_connection(conn_id)?;
            }
        }
        
        Ok(())
    }
    
    /// Process received data
    fn process_data(&mut self, conn_id: usize) -> ServerResult<()> {
        // Check if we have a connection
        if !self.connections.contains_key(&conn_id) {
            return Ok(());
        }
        
        // We need to clone the buffer data to avoid borrow checker conflicts
        let buffer_data = {
            let connection = self.connections.get(&conn_id).unwrap();
            let buffer = connection.buffer();
            buffer.slice().to_vec()
        };
        
        // Now parse the data
        {
            let parser = self.parsers.get_mut(&conn_id).unwrap();
            parser.parse(&buffer_data)?;
            
            // If we don't have a complete request, return early
            if !parser.is_complete() {
                return Ok(());
            }
            
            // Get the request before we borrow self again
            let request = parser.get_request()?;
            
            
            // Clone the request to avoid borrow issues
            let request_clone = request.clone();
            
            // Reset the parser early to release the mutable borrow
            parser.reset();
            
            // Get the response (here we use &self, not &mut self)
            let response = self.handle_request(&request_clone)?;
            
            // Now we can encode the response outside of any borrows
            let mut encoded = Vec::new();
            response.serialize(&mut encoded)?;
            
            
            // Finally get a mutable reference to the connection
            let connection = self.connections.get_mut(&conn_id).unwrap();
            connection.set_state(ConnectionState::Processing);
            connection.buffer_mut().write(&encoded)?;
            connection.set_state(ConnectionState::Writing);
            
            // Immediately try to write the response to the TCP stream
            self.handle_write(conn_id)?;
        }
        
        Ok(())
    }
    
    /// Handle a write event
    fn handle_write(&mut self, conn_id: usize) -> ServerResult<()> {
        let connection = match self.connections.get_mut(&conn_id) {
            Some(conn) => conn,
            None => return Ok(()),
        };
        
        // Check conditions before taking mutable references
        let should_write = connection.state() == ConnectionState::Writing && 
                          connection.buffer().available_data() > 0;
        
        if should_write {
            // Create a temporary buffer to hold data we'll write
            let data_to_write = connection.buffer().slice().to_vec();
            
            // Now write that buffer to the stream
            match connection.stream_mut().write(&data_to_write) {
                Ok(0) => {
                    // Connection closed
                    connection.set_state(ConnectionState::Closed);
                    // Return first, then close after we release the mutable borrow
                    return self.close_connection(conn_id);
                }
                Ok(bytes_written) => {
                    // Update the buffer position by advancing the read position
                    if let Err(e) = connection.buffer_mut().advance_read(bytes_written) {
                        println!("Error advancing buffer read position: {}", e);
                        connection.set_state(ConnectionState::Closed);
                        return self.close_connection(conn_id);
                    }
                    
                    // If no more data to write, we're done with this request
                    if connection.buffer().available_data() == 0 {
                        // Check if we're keeping the connection alive
                        connection.set_state(ConnectionState::Reading);
                    }
                }
                Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
                    // Would block, try again later
                }
                Err(e) => {
                    // Error writing
                    println!("Error writing to connection {}: {}", conn_id, e);
                    connection.set_state(ConnectionState::Closed);
                    return self.close_connection(conn_id);
                }
            }
        }
        
        Ok(())
    }
    
    /// Close a connection
    fn close_connection(&mut self, conn_id: usize) -> ServerResult<()> {
        if let Some(mut conn) = self.connections.remove(&conn_id) {
            self.poller.deregister(&conn)?;
            let _ = conn.close();
        }
        
        self.parsers.remove(&conn_id);
        
        Ok(())
    }
    
    /// Check for timed out connections
    fn check_timeouts(&mut self) -> ServerResult<()> {
        let now = Instant::now();
        let timed_out: Vec<usize> = self.connections
            .iter()
            .filter(|(_, conn)| conn.is_timed_out())
            .map(|(id, _)| *id)
            .collect();
        
        for conn_id in timed_out {
            println!("Connection {} timed out", conn_id);
            self.close_connection(conn_id)?;
        }
        
        Ok(())
    }
    
    /// Handle an HTTP request
    fn handle_request(&self, request: &Request) -> ServerResult<Response> {
        // If we have a router set, use it to handle the request
        if let Some(router) = &self.router {
            router.handle_request(request)
        } else if let Some(middleware_chain) = &self.middleware_chain {
            // If we have a middleware chain set, use it to handle the request
            middleware_chain.handle(request)
        } else {
            // Default handler - just return a simple 200 OK response
            let mut response = Response::new(Status::Ok);
            response.set_body("Hello, World!\n".as_bytes());
            
            Ok(response)
        }
    }
}