use crate::error::ServerResult;
use crate::http::{Method, Request, Response, Status};
use std::collections::HashMap;
use std::sync::Arc;
use std::fmt;

/// A handler function for processing HTTP requests
pub type HandlerFn = Arc<dyn Fn(&Request) -> ServerResult<Response> + Send + Sync>;

/// A route entry in the router
#[derive(Clone)]
struct RouteEntry {
    /// The HTTP method this route responds to
    method: Method,
    
    /// The path pattern for this route
    path: String,
    
    /// The handler function for this route
    handler: HandlerFn,
}

// Custom Debug implementation for RouteEntry since handler can't be automatically derived
impl fmt::Debug for RouteEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RouteEntry")
            .field("method", &self.method)
            .field("path", &self.path)
            .field("handler", &"<function>")
            .finish()
    }
}

/// A router for HTTP requests
#[derive(Clone)]
pub struct Router {
    /// The routes registered with this router
    routes: Vec<RouteEntry>,
    
    /// The handler to use when no route matches
    not_found_handler: HandlerFn,
}

// Custom Debug implementation for Router
impl fmt::Debug for Router {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Router")
            .field("routes", &self.routes)
            .field("not_found_handler", &"<function>")
            .finish()
    }
}

impl Router {
    /// Create a new router
    pub fn new() -> Self {
        // Default 404 handler
        let not_found_handler: HandlerFn = Arc::new(|req| {
            let mut response = Response::new(Status::NotFound);
            response.set_body(format!("Not Found: {}", req.uri).as_bytes());
            Ok(response)
        });
        
        Self {
            routes: Vec::new(),
            not_found_handler,
        }
    }
    
    /// Add a route to the router
    pub fn add_route<F>(&mut self, method: Method, path: &str, handler: F) -> &mut Self
    where
        F: Fn(&Request) -> ServerResult<Response> + Send + Sync + 'static,
    {
        self.routes.push(RouteEntry {
            method,
            path: path.to_string(),
            handler: Arc::new(handler),
        });
        
        self
    }
    
    /// Add a GET route
    pub fn get<F>(&mut self, path: &str, handler: F) -> &mut Self
    where
        F: Fn(&Request) -> ServerResult<Response> + Send + Sync + 'static,
    {
        self.add_route(Method::Get, path, handler)
    }
    
    /// Add a POST route
    pub fn post<F>(&mut self, path: &str, handler: F) -> &mut Self
    where
        F: Fn(&Request) -> ServerResult<Response> + Send + Sync + 'static,
    {
        self.add_route(Method::Post, path, handler)
    }
    
    /// Add a PUT route
    pub fn put<F>(&mut self, path: &str, handler: F) -> &mut Self
    where
        F: Fn(&Request) -> ServerResult<Response> + Send + Sync + 'static,
    {
        self.add_route(Method::Put, path, handler)
    }
    
    /// Add a DELETE route
    pub fn delete<F>(&mut self, path: &str, handler: F) -> &mut Self
    where
        F: Fn(&Request) -> ServerResult<Response> + Send + Sync + 'static,
    {
        self.add_route(Method::Delete, path, handler)
    }
    
    /// Set the not found handler
    pub fn set_not_found_handler<F>(&mut self, handler: F) -> &mut Self
    where
        F: Fn(&Request) -> ServerResult<Response> + Send + Sync + 'static,
    {
        self.not_found_handler = Arc::new(handler);
        self
    }
    
    /// Handle a request
    pub fn handle_request(&self, request: &Request) -> ServerResult<Response> {
        // Simple path matching for now - just exact matches
        // A more advanced implementation would use a trie or radix tree
        for route in &self.routes {
            if route.method == request.method && self.path_matches(&route.path, &request.uri) {
                return (route.handler)(request);
            }
        }
        
        // No route matched, use the not found handler
        (self.not_found_handler)(request)
    }
    
    /// Check if a path matches a route pattern
    fn path_matches(&self, pattern: &str, path: &str) -> bool {
        // Simple matching for now
        // This could be extended to support path parameters and wildcards
        
        // Check for exact match
        if pattern == path {
            return true;
        }
        
        // Check for wildcard match at end (e.g., "/users/*")
        if pattern.ends_with('*') {
            let prefix = &pattern[0..pattern.len() - 1];
            return path.starts_with(prefix);
        }
        
        // Check for path parameter match (e.g., "/users/:id")
        // For simplicity, we'll just check if the segments match in number and non-param segments match exactly
        let pattern_segments: Vec<&str> = pattern.split('/').filter(|s| !s.is_empty()).collect();
        let path_segments: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
        
        if pattern_segments.len() != path_segments.len() {
            return false;
        }
        
        for (i, pattern_seg) in pattern_segments.iter().enumerate() {
            if !pattern_seg.starts_with(':') && pattern_seg != &path_segments[i] {
                return false;
            }
        }
        
        true
    }
    
    /// Extract path parameters from a request URI based on a route pattern
    pub fn extract_params(&self, pattern: &str, path: &str) -> HashMap<String, String> {
        let mut params = HashMap::new();
        
        // If not a parametrized path, return empty map
        if !pattern.contains(':') {
            return params;
        }
        
        let pattern_segments: Vec<&str> = pattern.split('/').filter(|s| !s.is_empty()).collect();
        let path_segments: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
        
        if pattern_segments.len() != path_segments.len() {
            return params;
        }
        
        for (i, pattern_seg) in pattern_segments.iter().enumerate() {
            if pattern_seg.starts_with(':') {
                let param_name = &pattern_seg[1..];
                let param_value = path_segments[i];
                params.insert(param_name.to_string(), param_value.to_string());
            }
        }
        
        params
    }
}

impl Default for Router {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_router_exact_match() {
        let mut router = Router::new();
        
        router.get("/", |_| {
            let mut response = Response::new(Status::Ok);
            response.set_body(b"Home");
            Ok(response)
        });
        
        router.get("/users", |_| {
            let mut response = Response::new(Status::Ok);
            response.set_body(b"Users");
            Ok(response)
        });
        
        // Test home route
        let request = Request::new(Method::Get, "/");
        let response = router.handle_request(&request).unwrap();
        assert_eq!(response.status, Status::Ok);
        assert_eq!(response.body, b"Home");
        
        // Test users route
        let request = Request::new(Method::Get, "/users");
        let response = router.handle_request(&request).unwrap();
        assert_eq!(response.status, Status::Ok);
        assert_eq!(response.body, b"Users");
        
        // Test 404
        let request = Request::new(Method::Get, "/not-found");
        let response = router.handle_request(&request).unwrap();
        assert_eq!(response.status, Status::NotFound);
    }
    
    #[test]
    fn test_router_method_matching() {
        let mut router = Router::new();
        
        router.get("/api", |_| {
            let mut response = Response::new(Status::Ok);
            response.set_body(b"GET");
            Ok(response)
        });
        
        router.post("/api", |_| {
            let mut response = Response::new(Status::Ok);
            response.set_body(b"POST");
            Ok(response)
        });
        
        // Test GET
        let request = Request::new(Method::Get, "/api");
        let response = router.handle_request(&request).unwrap();
        assert_eq!(response.status, Status::Ok);
        assert_eq!(response.body, b"GET");
        
        // Test POST
        let request = Request::new(Method::Post, "/api");
        let response = router.handle_request(&request).unwrap();
        assert_eq!(response.status, Status::Ok);
        assert_eq!(response.body, b"POST");
        
        // Test other method (not found)
        let request = Request::new(Method::Put, "/api");
        let response = router.handle_request(&request).unwrap();
        assert_eq!(response.status, Status::NotFound);
    }
    
    #[test]
    fn test_router_params() {
        let router = Router::new();
        
        let params = router.extract_params("/users/:id", "/users/123");
        assert_eq!(params.len(), 1);
        assert_eq!(params.get("id").unwrap(), "123");
        
        let params = router.extract_params("/users/:id/posts/:post_id", "/users/123/posts/456");
        assert_eq!(params.len(), 2);
        assert_eq!(params.get("id").unwrap(), "123");
        assert_eq!(params.get("post_id").unwrap(), "456");
        
        let params = router.extract_params("/users", "/users");
        assert_eq!(params.len(), 0);
    }
}