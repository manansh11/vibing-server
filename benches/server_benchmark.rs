use criterion::{black_box, criterion_group, criterion_main, Criterion};
use high_performance_server::buffer::Buffer;
use high_performance_server::http::{HttpParser, Method, Request, Response, Status};
use high_performance_server::memory::MemoryManager;
use std::io::{Cursor, Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

fn benchmark_buffer_read_write(c: &mut Criterion) {
    let mut group = c.benchmark_group("buffer");
    
    group.bench_function("read_write_small", |b| {
        b.iter(|| {
            let mut buffer = Buffer::new(1024);
            let data = black_box(vec![0; 256]);
            let mut cursor = Cursor::new(&data);
            
            buffer.read_from(&mut cursor).unwrap();
            let mut output = Vec::new();
            buffer.write_to(&mut output).unwrap();
            
            assert_eq!(data, output);
        })
    });
    
    group.bench_function("read_write_large", |b| {
        b.iter(|| {
            let mut buffer = Buffer::new(8192);
            let data = black_box(vec![0; 4096]);
            let mut cursor = Cursor::new(&data);
            
            buffer.read_from(&mut cursor).unwrap();
            let mut output = Vec::new();
            buffer.write_to(&mut output).unwrap();
            
            assert_eq!(data, output);
        })
    });
    
    group.finish();
}

fn benchmark_http_parsing(c: &mut Criterion) {
    let mut group = c.benchmark_group("http_parser");
    
    let simple_request = "GET / HTTP/1.1\r\nHost: localhost\r\n\r\n";
    
    group.bench_function("parse_simple_request", |b| {
        b.iter(|| {
            let mut parser = HttpParser::new();
            parser.parse(simple_request.as_bytes()).unwrap();
            assert!(parser.is_complete());
            let request = parser.get_request().unwrap();
            assert_eq!(request.method, Method::Get);
            assert_eq!(request.uri, "/");
        })
    });
    
    let complex_request = "POST /api/items HTTP/1.1\r\n\
                          Host: example.com\r\n\
                          Content-Type: application/json\r\n\
                          Content-Length: 27\r\n\
                          User-Agent: Benchmark\r\n\
                          Accept: */*\r\n\
                          \r\n\
                          {\"name\":\"test\",\"value\":123}";
    
    group.bench_function("parse_complex_request", |b| {
        b.iter(|| {
            let mut parser = HttpParser::new();
            parser.parse(complex_request.as_bytes()).unwrap();
            assert!(parser.is_complete());
            let request = parser.get_request().unwrap();
            assert_eq!(request.method, Method::Post);
            assert_eq!(request.uri, "/api/items");
            assert_eq!(request.body.len(), 27);
        })
    });
    
    group.finish();
}

fn benchmark_memory_pool(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory_pool");
    
    group.bench_function("allocate_deallocate_small", |b| {
        b.iter(|| {
            let memory_manager = MemoryManager::new();
            let mut handles = Vec::with_capacity(100);
            
            for _ in 0..100 {
                handles.push(memory_manager.allocate(64).unwrap());
            }
            
            // Using the memory
            for handle in &mut handles {
                let slice = handle.as_slice_mut();
                slice[0] = 1;
            }
            
            // handles are automatically deallocated when dropped
        })
    });
    
    group.bench_function("allocate_deallocate_mixed", |b| {
        b.iter(|| {
            let memory_manager = MemoryManager::new();
            let mut handles = Vec::with_capacity(100);
            
            for i in 0..100 {
                // Mix of different sizes
                let size = match i % 4 {
                    0 => 32,
                    1 => 64,
                    2 => 128,
                    _ => 256,
                };
                
                handles.push(memory_manager.allocate(size).unwrap());
            }
            
            // Using the memory
            for handle in &mut handles {
                let slice = handle.as_slice_mut();
                slice[0] = 1;
            }
            
            // handles are automatically deallocated when dropped
        })
    });
    
    group.finish();
}

fn benchmark_response_serialization(c: &mut Criterion) {
    let mut group = c.benchmark_group("response");
    
    group.bench_function("simple_response", |b| {
        b.iter(|| {
            let mut response = Response::new(Status::Ok);
            response.set_body("Hello, World!".as_bytes());
            
            let mut buffer = Vec::new();
            response.serialize(&mut buffer).unwrap();
            
            assert!(buffer.len() > 0);
        })
    });
    
    group.bench_function("complex_response", |b| {
        b.iter(|| {
            let mut response = Response::new(Status::Ok);
            response.set_header("Content-Type", "application/json");
            response.set_header("Cache-Control", "no-cache");
            response.set_header("X-Custom-Header", "Benchmark");
            response.set_body("{\"status\":\"success\",\"data\":{\"items\":[1,2,3,4,5]}}".as_bytes());
            
            let mut buffer = Vec::new();
            response.serialize(&mut buffer).unwrap();
            
            assert!(buffer.len() > 0);
        })
    });
    
    group.finish();
}

criterion_group!(
    benches,
    benchmark_buffer_read_write,
    benchmark_http_parsing,
    benchmark_memory_pool,
    benchmark_response_serialization
);
criterion_main!(benches);