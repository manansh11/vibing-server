use high_performance_server::http::{HttpParser, Method, Request, Response, Status};
use std::io::Cursor;

#[test]
fn test_http_parser_simple_get() {
    let mut parser = HttpParser::new();
    let request_data = b"GET /index.html HTTP/1.1\r\nHost: example.com\r\n\r\n";
    
    parser.parse(request_data).unwrap();
    assert!(parser.is_complete());
    
    let request = parser.get_request().unwrap();
    assert_eq!(request.method, Method::Get);
    assert_eq!(request.uri, "/index.html");
    assert_eq!(request.headers.get("host").unwrap(), "example.com");
    assert_eq!(request.body.len(), 0);
}

#[test]
fn test_http_parser_post_with_body() {
    let mut parser = HttpParser::new();
    let request_data = b"POST /submit HTTP/1.1\r\nHost: example.com\r\nContent-Type: application/json\r\nContent-Length: 15\r\n\r\n{\"name\":\"test\"}";
    
    println!("Request data length: {}", request_data.len());
    println!("Request data as string: {:?}", std::str::from_utf8(request_data).unwrap());
    
    for (i, &b) in request_data.iter().enumerate() {
        println!("byte[{}] = {}, char: {}", i, b, if b >= 32 && b <= 126 { b as char } else { ' ' });
    }
    
    parser.parse(request_data).unwrap();
    println!("Parser state: {:?}", parser.state);
    println!("Content-Length: {}", parser.content_length);
    println!("Body length: {}", parser.body.len());
    println!("Body content: {:?}", std::str::from_utf8(&parser.body).unwrap_or("Invalid UTF-8"));
    
    assert!(parser.is_complete());
    
    let request = parser.get_request().unwrap();
    assert_eq!(request.method, Method::Post);
    assert_eq!(request.uri, "/submit");
    assert_eq!(request.headers.get("content-type").unwrap(), "application/json");
    assert_eq!(request.headers.get("content-length").unwrap(), "15");
    assert_eq!(request.body, b"{\"name\":\"test\"}");
}

#[test]
fn test_http_parser_multiple_headers() {
    let mut parser = HttpParser::new();
    let request_data = b"GET /api/data HTTP/1.1\r\n\
                        Host: example.com\r\n\
                        User-Agent: Test Client\r\n\
                        Accept: application/json\r\n\
                        Accept-Language: en-US\r\n\
                        Cookie: session=abc123\r\n\
                        \r\n";
    
    parser.parse(request_data).unwrap();
    assert!(parser.is_complete());
    
    let request = parser.get_request().unwrap();
    assert_eq!(request.method, Method::Get);
    assert_eq!(request.uri, "/api/data");
    assert_eq!(request.headers.get("host").unwrap(), "example.com");
    assert_eq!(request.headers.get("user-agent").unwrap(), "Test Client");
    assert_eq!(request.headers.get("accept").unwrap(), "application/json");
    assert_eq!(request.headers.get("accept-language").unwrap(), "en-US");
    assert_eq!(request.headers.get("cookie").unwrap(), "session=abc123");
}

#[test]
fn test_http_parser_reset() {
    let mut parser = HttpParser::new();
    
    // Parse a first request
    parser.parse(b"GET / HTTP/1.1\r\nHost: example.com\r\n\r\n").unwrap();
    assert!(parser.is_complete());
    
    // Reset the parser
    parser.reset();
    assert!(!parser.is_complete());
    
    // Parse a second request
    parser.parse(b"POST / HTTP/1.1\r\nHost: example.com\r\n\r\n").unwrap();
    assert!(parser.is_complete());
    
    let request = parser.get_request().unwrap();
    assert_eq!(request.method, Method::Post);
}

#[test]
fn test_request_methods() {
    let mut request = Request::new(Method::Get, "/api/data");
    
    request.set_header("Content-Type", "application/json");
    request.set_body(b"{\"query\":\"test\"}");
    
    assert_eq!(request.method, Method::Get);
    assert_eq!(request.uri, "/api/data");
    assert_eq!(request.get_header("content-type").unwrap(), "application/json");
    assert_eq!(request.get_header("content-length").unwrap(), "16");
    assert_eq!(request.body, b"{\"query\":\"test\"}");
}

#[test]
fn test_response_creation_and_serialization() {
    let mut response = Response::new(Status::Ok);
    response.set_header("Content-Type", "text/plain");
    response.set_body(b"Hello, World!");
    
    let mut buffer = Vec::new();
    response.serialize(&mut buffer).unwrap();
    
    let response_str = String::from_utf8_lossy(&buffer);
    assert!(response_str.starts_with("HTTP/1.1 200 OK\r\n"));
    assert!(response_str.contains("Content-Type: text/plain\r\n"));
    assert!(response_str.contains("Content-Length: 13\r\n"));
    assert!(response_str.ends_with("\r\n\r\nHello, World!"));
}

#[test]
fn test_different_status_codes() {
    let statuses = vec![
        (Status::Ok, 200, "OK"),
        (Status::Created, 201, "Created"),
        (Status::BadRequest, 400, "Bad Request"),
        (Status::NotFound, 404, "Not Found"),
        (Status::InternalServerError, 500, "Internal Server Error"),
    ];
    
    for (status, code, text) in statuses {
        let response = Response::new(status);
        
        let mut buffer = Vec::new();
        response.serialize(&mut buffer).unwrap();
        
        let response_str = String::from_utf8_lossy(&buffer);
        assert!(response_str.starts_with(&format!("HTTP/1.1 {} {}\r\n", code, text)));
    }
}