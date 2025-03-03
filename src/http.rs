use crate::error::{ServerError, ServerResult};
use std::collections::HashMap;
use std::io::Write;
use std::str;

/// HTTP Status Codes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    Continue = 100,
    SwitchingProtocols = 101,
    
    Ok = 200,
    Created = 201,
    Accepted = 202,
    NoContent = 204,
    
    MovedPermanently = 301,
    Found = 302,
    NotModified = 304,
    
    BadRequest = 400,
    Unauthorized = 401,
    Forbidden = 403,
    NotFound = 404,
    MethodNotAllowed = 405,
    RequestTimeout = 408,
    PayloadTooLarge = 413,
    
    InternalServerError = 500,
    NotImplemented = 501,
    BadGateway = 502,
    ServiceUnavailable = 503,
}

impl Status {
    /// Get the text description for this status code
    pub fn as_str(&self) -> &'static str {
        match *self {
            Status::Continue => "Continue",
            Status::SwitchingProtocols => "Switching Protocols",
            
            Status::Ok => "OK",
            Status::Created => "Created",
            Status::Accepted => "Accepted",
            Status::NoContent => "No Content",
            
            Status::MovedPermanently => "Moved Permanently",
            Status::Found => "Found",
            Status::NotModified => "Not Modified",
            
            Status::BadRequest => "Bad Request",
            Status::Unauthorized => "Unauthorized",
            Status::Forbidden => "Forbidden",
            Status::NotFound => "Not Found",
            Status::MethodNotAllowed => "Method Not Allowed",
            Status::RequestTimeout => "Request Timeout",
            Status::PayloadTooLarge => "Payload Too Large",
            
            Status::InternalServerError => "Internal Server Error",
            Status::NotImplemented => "Not Implemented",
            Status::BadGateway => "Bad Gateway",
            Status::ServiceUnavailable => "Service Unavailable",
        }
    }
}

/// HTTP Methods
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Method {
    Get,
    Head,
    Post,
    Put,
    Delete,
    Options,
    Trace,
    Connect,
    Patch,
}

impl Method {
    /// Parse a method from a string
    pub fn from_str(s: &str) -> ServerResult<Self> {
        match s {
            "GET" => Ok(Method::Get),
            "HEAD" => Ok(Method::Head),
            "POST" => Ok(Method::Post),
            "PUT" => Ok(Method::Put),
            "DELETE" => Ok(Method::Delete),
            "OPTIONS" => Ok(Method::Options),
            "TRACE" => Ok(Method::Trace),
            "CONNECT" => Ok(Method::Connect),
            "PATCH" => Ok(Method::Patch),
            _ => Err(ServerError::HttpParse(format!("Invalid method: {}", s))),
        }
    }
    
    /// Convert the method to a string
    pub fn as_str(&self) -> &'static str {
        match *self {
            Method::Get => "GET",
            Method::Head => "HEAD",
            Method::Post => "POST",
            Method::Put => "PUT",
            Method::Delete => "DELETE",
            Method::Options => "OPTIONS",
            Method::Trace => "TRACE",
            Method::Connect => "CONNECT",
            Method::Patch => "PATCH",
        }
    }
}

/// HTTP Parser State
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HttpParserState {
    RequestLine,
    Headers,
    Body,
    Complete,
}

/// HTTP Parser
pub struct HttpParser {
    pub state: HttpParserState,
    pub method: Option<Method>,
    pub uri: Option<String>,
    pub version: Option<String>,
    pub headers: HashMap<String, String>,
    pub body: Vec<u8>,
    pub content_length: usize,
}

impl HttpParser {
    /// Create a new HTTP parser
    pub fn new() -> Self {
        Self {
            state: HttpParserState::RequestLine,
            method: None,
            uri: None,
            version: None,
            headers: HashMap::new(),
            body: Vec::new(),
            content_length: 0,
        }
    }
    
    /// Parse a chunk of data
    pub fn parse(&mut self, data: &[u8]) -> ServerResult<()> {
        // If we're already complete, reset
        if self.state == HttpParserState::Complete {
            self.reset();
        }
        
        // Convert to string for header parsing
        let data_str = match str::from_utf8(data) {
            Ok(s) => s,
            Err(_) => return Err(ServerError::HttpParse("Invalid UTF-8".to_string())),
        };
        
        // Find the end of headers marker
        if let Some(headers_end) = data_str.find("\r\n\r\n") {
            let headers_part = &data_str[0..headers_end];
            
            // Process headers section line by line
            let lines: Vec<&str> = headers_part.split("\r\n").collect();
            if !lines.is_empty() {
                // Handle request line (first line)
                if self.state == HttpParserState::RequestLine {
                    self.parse_request_line(lines[0])?;
                    self.state = HttpParserState::Headers;
                }
                
                // Parse headers (subsequent lines)
                if self.state == HttpParserState::Headers {
                    for line in &lines[1..] {
                        if !line.is_empty() {
                            self.parse_header(line)?;
                        }
                    }
                    
                    // Check for content length
                    if let Some(content_length) = self.headers.get("content-length") {
                        self.content_length = content_length.parse().unwrap_or(0);
                    }
                    
                    // Body starts after headers end marker
                    let body_start = headers_end + 4; // +4 for \r\n\r\n
                    
                    if self.content_length > 0 && body_start < data.len() {
                        // Add body data
                        self.body.extend_from_slice(&data[body_start..]);
                        
                        // Check if we have the complete body
                        if self.body.len() >= self.content_length {
                            // Trim any excess data
                            if self.body.len() > self.content_length {
                                self.body.truncate(self.content_length);
                            }
                            self.state = HttpParserState::Complete;
                        } else {
                            self.state = HttpParserState::Body;
                        }
                    } else if self.content_length == 0 {
                        // No body expected
                        self.state = HttpParserState::Complete;
                    } else {
                        // Expecting body but none in this chunk
                        self.state = HttpParserState::Body;
                    }
                }
            }
        } else if self.state == HttpParserState::Body {
            // We're in body state but didn't get the headers part in this chunk
            // Just add everything to body
            self.body.extend_from_slice(data);
            
            // Check if we now have the complete body
            if self.body.len() >= self.content_length {
                // Trim any excess data
                if self.body.len() > self.content_length {
                    self.body.truncate(self.content_length);
                }
                self.state = HttpParserState::Complete;
            }
        }
        
        Ok(())
    }
    
    /// Parse a request line
    fn parse_request_line(&mut self, line: &str) -> ServerResult<()> {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() != 3 {
            return Err(ServerError::HttpParse(
                "Invalid request line".to_string(),
            ));
        }
        
        self.method = Some(Method::from_str(parts[0])?);
        self.uri = Some(parts[1].to_string());
        self.version = Some(parts[2].to_string());
        
        Ok(())
    }
    
    /// Parse a header line
    fn parse_header(&mut self, line: &str) -> ServerResult<()> {
        if let Some(colon_idx) = line.find(':') {
            let key = line[..colon_idx].trim().to_lowercase();
            let value = line[colon_idx + 1..].trim().to_string();
            self.headers.insert(key, value);
            Ok(())
        } else {
            Err(ServerError::HttpParse("Invalid header".to_string()))
        }
    }
    
    /// Check if the parser has completed parsing a request
    pub fn is_complete(&self) -> bool {
        self.state == HttpParserState::Complete
    }
    
    /// Reset the parser for a new request
    pub fn reset(&mut self) {
        self.state = HttpParserState::RequestLine;
        self.method = None;
        self.uri = None;
        self.version = None;
        self.headers.clear();
        self.body.clear();
        self.content_length = 0;
    }
    
    /// Get the parsed request
    pub fn get_request(&self) -> ServerResult<Request> {
        if !self.is_complete() {
            return Err(ServerError::HttpParse(
                "Request not complete".to_string(),
            ));
        }
        
        let method = self.method.ok_or_else(|| {
            ServerError::HttpParse("Method not set".to_string())
        })?;
        
        let uri = self.uri.as_ref().ok_or_else(|| {
            ServerError::HttpParse("URI not set".to_string())
        })?.clone();
        
        // Parse query parameters if present
        let mut query_params = HashMap::new();
        if let Some(query_start) = uri.find('?') {
            let query = &uri[query_start + 1..];
            for pair in query.split('&') {
                if let Some(eq_pos) = pair.find('=') {
                    let (key, value) = pair.split_at(eq_pos);
                    query_params.insert(key.to_string(), value[1..].to_string());
                } else {
                    query_params.insert(pair.to_string(), "".to_string());
                }
            }
        }
        
        Ok(Request {
            method,
            uri,
            headers: self.headers.clone(),
            body: self.body.clone(),
            query_params,
        })
    }
}

/// HTTP Request
#[derive(Debug, Clone)]
pub struct Request {
    pub method: Method,
    pub uri: String,
    pub headers: HashMap<String, String>,
    pub body: Vec<u8>,
    /// Query parameters parsed from the URI
    pub query_params: HashMap<String, String>,
}

impl Request {
    /// Create a new request
    pub fn new(method: Method, uri: &str) -> Self {
        // Parse query parameters if present
        let mut query_params = HashMap::new();
        let (path, query) = match uri.find('?') {
            Some(pos) => {
                let (path, query) = uri.split_at(pos);
                (path, Some(&query[1..]))
            }
            None => (uri, None),
        };
        
        // Parse query parameters if present
        if let Some(query) = query {
            for pair in query.split('&') {
                if let Some(pos) = pair.find('=') {
                    let (key, value) = pair.split_at(pos);
                    query_params.insert(key.to_string(), value[1..].to_string());
                } else {
                    query_params.insert(pair.to_string(), "".to_string());
                }
            }
        }
        
        Self {
            method,
            uri: uri.to_string(),
            headers: HashMap::new(),
            body: Vec::new(),
            query_params,
        }
    }
    
    /// Set a header
    pub fn set_header(&mut self, name: &str, value: &str) {
        self.headers.insert(name.to_lowercase(), value.to_string());
    }
    
    /// Get a header
    pub fn get_header(&self, name: &str) -> Option<&String> {
        self.headers.get(&name.to_lowercase())
    }
    
    /// Set the body
    pub fn set_body(&mut self, body: &[u8]) {
        self.body = body.to_vec();
        self.set_header("Content-Length", &self.body.len().to_string());
    }
}

/// HTTP Response
#[derive(Debug, Clone)]
pub struct Response {
    pub status: Status,
    pub headers: HashMap<String, String>,
    pub body: Vec<u8>,
}

impl Response {
    /// Create a new response
    pub fn new(status: Status) -> Self {
        let mut headers = HashMap::new();
        headers.insert("Server".to_string(), "High-Performance-Server/0.1".to_string());
        headers.insert("Connection".to_string(), "close".to_string());
        
        Self {
            status,
            headers,
            body: Vec::new(),
        }
    }
    
    /// Set a header
    pub fn set_header(&mut self, name: &str, value: &str) {
        self.headers.insert(name.to_string(), value.to_string());
    }
    
    /// Set the body and update content-length
    pub fn set_body(&mut self, body: &[u8]) {
        self.body = body.to_vec();
        self.set_header("Content-Length", &body.len().to_string());
        self.set_header("Content-Type", "text/plain");
    }
    
    /// Serialize the response to a byte vector
    pub fn serialize(&self, writer: &mut Vec<u8>) -> ServerResult<()> {
        // Write status line
        write!(writer, "HTTP/1.1 {} {}\r\n", self.status as u16, self.status.as_str())
            .map_err(|e| ServerError::Io(e))?;
        
        // Write headers
        for (name, value) in &self.headers {
            write!(writer, "{}: {}\r\n", name, value)
                .map_err(|e| ServerError::Io(e))?;
        }
        
        // Write blank line
        write!(writer, "\r\n").map_err(|e| ServerError::Io(e))?;
        
        // Write body
        writer.extend_from_slice(&self.body);
        
        Ok(())
    }
}