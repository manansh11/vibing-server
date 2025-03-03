use crate::error::ServerResult;
use crate::http::{Request, Response};
use std::sync::Arc;
use std::time::Instant;

/// A middleware function for processing HTTP requests and responses
pub type MiddlewareFn = Arc<dyn Fn(&Request, MiddlewareNext) -> ServerResult<Response> + Send + Sync>;

/// The next middleware or handler function in the chain
pub type MiddlewareNext = Arc<dyn Fn(&Request) -> ServerResult<Response> + Send + Sync>;

/// A middleware chain for processing requests
pub struct MiddlewareChain {
    /// The middleware functions in the chain
    middleware: Vec<MiddlewareFn>,
    
    /// The final handler function
    handler: Option<MiddlewareNext>,
}

impl MiddlewareChain {
    /// Create a new middleware chain
    pub fn new() -> Self {
        Self {
            middleware: Vec::new(),
            handler: None,
        }
    }
    
    /// Add a middleware function to the chain
    pub fn add<F>(&mut self, middleware: F) -> &mut Self
    where
        F: Fn(&Request, MiddlewareNext) -> ServerResult<Response> + Send + Sync + 'static,
    {
        self.middleware.push(Arc::new(middleware));
        self
    }
    
    /// Set the final handler function
    pub fn set_handler<F>(&mut self, handler: F) -> &mut Self
    where
        F: Fn(&Request) -> ServerResult<Response> + Send + Sync + 'static,
    {
        self.handler = Some(Arc::new(handler));
        self
    }
    
    /// Process a request through the middleware chain
    pub fn handle(&self, request: &Request) -> ServerResult<Response> {
        if let Some(handler) = &self.handler {
            // Add explicit type annotation
            let chain: Vec<MiddlewareNext> = Vec::with_capacity(self.middleware.len());
            
            // Build the middleware chain in reverse order
            let mut next: MiddlewareNext = handler.clone();
            
            for middleware in self.middleware.iter().rev() {
                let current = middleware.clone();
                let prev_next = next.clone();
                
                next = Arc::new(move |req| {
                    current(req, prev_next.clone())
                });
            }
            
            // Execute the chain
            next(request)
        } else {
            Err(crate::error::ServerError::EventLoop(
                "No handler set for middleware chain".to_string(),
            ))
        }
    }
}

impl Default for MiddlewareChain {
    fn default() -> Self {
        Self::new()
    }
}

/// Common middleware functions

/// Logging middleware - logs information about requests and responses
pub fn logging_middleware(request: &Request, next: MiddlewareNext) -> ServerResult<Response> {
    let start_time = Instant::now();
    println!("[Request] {} {}", request.method.as_str(), request.uri);
    
    let response = next(request);
    
    let elapsed = start_time.elapsed();
    match &response {
        Ok(resp) => {
            println!(
                "[Response] {} {} - {} - {:?}",
                request.method.as_str(),
                request.uri,
                resp.status as u16,
                elapsed
            );
        }
        Err(e) => {
            println!(
                "[Error] {} {} - Error: {:?} - {:?}",
                request.method.as_str(),
                request.uri,
                e,
                elapsed
            );
        }
    }
    
    response
}

/// CORS middleware - adds CORS headers to responses
pub fn cors_middleware(allowed_origins: Vec<String>) -> impl Fn(&Request, MiddlewareNext) -> ServerResult<Response> + Send + Sync {
    move |request, next| {
        let mut response = next(request)?;
        
        // Check if the origin header is present and allowed
        if let Some(origin) = request.get_header("origin") {
            if allowed_origins.contains(origin) || allowed_origins.contains(&"*".to_string()) {
                response.set_header("Access-Control-Allow-Origin", origin);
                response.set_header("Access-Control-Allow-Methods", "GET, POST, PUT, DELETE");
                response.set_header("Access-Control-Allow-Headers", "Content-Type");
            }
        }
        
        Ok(response)
    }
}

/// Basic auth middleware - requires basic authentication for requests
pub fn basic_auth_middleware(
    username: String,
    password: String,
) -> impl Fn(&Request, MiddlewareNext) -> ServerResult<Response> + Send + Sync {
    move |request, next| {
        // Check for the Authorization header
        if let Some(auth) = request.get_header("authorization") {
            if auth.starts_with("Basic ") {
                // Extract the credentials
                let base64_credentials = &auth[6..];
                let decoded = base64::decode(base64_credentials);
                
                if let Ok(bytes) = decoded {
                    if let Ok(credentials) = String::from_utf8(bytes) {
                        // Check if the credentials match
                        let parts: Vec<&str> = credentials.split(':').collect();
                        if parts.len() == 2 && parts[0] == username && parts[1] == password {
                            return next(request);
                        }
                    }
                }
            }
        }
        
        // Authentication failed
        let mut response = Response::new(crate::http::Status::Unauthorized);
        response.set_header("WWW-Authenticate", "Basic realm=\"Server\"");
        response.set_body(b"Unauthorized");
        Ok(response)
    }
}

/// Content-type middleware - adds a default content-type header to responses
pub fn content_type_middleware(
    content_type: String,
) -> impl Fn(&Request, MiddlewareNext) -> ServerResult<Response> + Send + Sync {
    move |request, next| {
        let mut response = next(request)?;
        
        // Only add the Content-Type header if it's not already present
        if !response.headers.contains_key("Content-Type") {
            response.set_header("Content-Type", &content_type);
        }
        
        Ok(response)
    }
}

/// Compression middleware - compresses response bodies
pub fn compression_middleware(request: &Request, next: MiddlewareNext) -> ServerResult<Response> {
    let mut response = next(request)?;
    
    // Check if the client supports compression
    if let Some(accept_encoding) = request.get_header("accept-encoding") {
        if accept_encoding.contains("gzip") {
            // Only compress responses larger than a certain size
            if response.body.len() > 1024 {
                // Compress the body
                use flate2::write::GzEncoder;
                use flate2::Compression;
                use std::io::Write;
                
                let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
                encoder.write_all(&response.body)?;
                let compressed = encoder.finish()?;
                
                // Update the response
                response.body = compressed;
                response.set_header("Content-Encoding", "gzip");
                response.set_header("Content-Length", &response.body.len().to_string());
            }
        }
    }
    
    Ok(response)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::http::{Method, Status};
    
    #[test]
    fn test_middleware_chain() {
        let mut chain = MiddlewareChain::new();
        
        // Add a middleware that modifies the request
        chain.add(|request, next| {
            let mut modified_request = request.clone();
            modified_request.set_header("X-Modified", "true");
            next(&modified_request)
        });
        
        // Add a middleware that modifies the response
        chain.add(|request, next| {
            let mut response = next(request)?;
            response.set_header("X-Middleware", "applied");
            Ok(response)
        });
        
        // Set the final handler
        chain.set_handler(|request| {
            let mut response = Response::new(Status::Ok);
            response.set_body(b"Hello, World!");
            
            // Check if the request was modified
            if let Some(value) = request.get_header("x-modified") {
                if value == "true" {
                    response.set_header("X-Handler-Saw-Modified", "true");
                }
            }
            
            Ok(response)
        });
        
        // Process a request
        let request = Request::new(Method::Get, "/");
        let response = chain.handle(&request).unwrap();
        
        assert_eq!(response.status, Status::Ok);
        assert_eq!(response.body, b"Hello, World!");
        assert_eq!(response.headers.get("X-Middleware").unwrap(), "applied");
        assert_eq!(response.headers.get("X-Handler-Saw-Modified").unwrap(), "true");
    }
    
    #[test]
    fn test_logging_middleware() {
        let mut chain = MiddlewareChain::new();
        
        chain.add(logging_middleware);
        
        chain.set_handler(|_| {
            let mut response = Response::new(Status::Ok);
            response.set_body(b"Hello, World!");
            Ok(response)
        });
        
        let request = Request::new(Method::Get, "/");
        let response = chain.handle(&request).unwrap();
        
        assert_eq!(response.status, Status::Ok);
        assert_eq!(response.body, b"Hello, World!");
    }
    
    #[test]
    fn test_cors_middleware() {
        let mut chain = MiddlewareChain::new();
        
        chain.add(cors_middleware(vec!["http://example.com".to_string()]));
        
        chain.set_handler(|_| {
            let mut response = Response::new(Status::Ok);
            response.set_body(b"Hello, World!");
            Ok(response)
        });
        
        // Test with a valid origin
        let mut request = Request::new(Method::Get, "/");
        request.set_header("Origin", "http://example.com");
        let response = chain.handle(&request).unwrap();
        
        assert_eq!(response.status, Status::Ok);
        assert_eq!(
            response.headers.get("Access-Control-Allow-Origin").unwrap(),
            "http://example.com"
        );
        
        // Test with an invalid origin
        let mut request = Request::new(Method::Get, "/");
        request.set_header("Origin", "http://evil.com");
        let response = chain.handle(&request).unwrap();
        
        assert_eq!(response.status, Status::Ok);
        assert!(response.headers.get("Access-Control-Allow-Origin").is_none());
    }
    
    #[test]
    fn test_basic_auth_middleware() {
        let mut chain = MiddlewareChain::new();
        
        chain.add(basic_auth_middleware("admin".to_string(), "password".to_string()));
        
        chain.set_handler(|_| {
            let mut response = Response::new(Status::Ok);
            response.set_body(b"Secret data");
            Ok(response)
        });
        
        // Test without auth
        let request = Request::new(Method::Get, "/");
        let response = chain.handle(&request).unwrap();
        
        assert_eq!(response.status, Status::Unauthorized);
        
        // Test with correct auth
        let credentials = base64::encode("admin:password");
        let mut request = Request::new(Method::Get, "/");
        request.set_header("Authorization", &format!("Basic {}", credentials));
        let response = chain.handle(&request).unwrap();
        
        assert_eq!(response.status, Status::Ok);
        assert_eq!(response.body, b"Secret data");
        
        // Test with incorrect auth
        let credentials = base64::encode("admin:wrong");
        let mut request = Request::new(Method::Get, "/");
        request.set_header("Authorization", &format!("Basic {}", credentials));
        let response = chain.handle(&request).unwrap();
        
        assert_eq!(response.status, Status::Unauthorized);
    }
}