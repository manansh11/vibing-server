use high_performance_server::buffer::Buffer;
use std::io::Cursor;

#[test]
fn test_buffer_creation() {
    let buffer = Buffer::new(1024);
    assert_eq!(buffer.capacity(), 1024);
    assert_eq!(buffer.available_data(), 0);
    assert_eq!(buffer.remaining_capacity(), 1024);
}

#[test]
fn test_buffer_write() {
    let mut buffer = Buffer::new(1024);
    let data = b"Hello, World!";
    
    let bytes_written = buffer.write(data).unwrap();
    assert_eq!(bytes_written, data.len());
    assert_eq!(buffer.available_data(), data.len());
    assert_eq!(buffer.remaining_capacity(), 1024 - data.len());
    assert_eq!(buffer.slice(), data);
}

#[test]
fn test_buffer_read() {
    let mut buffer = Buffer::new(1024);
    let data = b"Hello, World!";
    buffer.write(data).unwrap();
    
    let mut read_data = vec![0; data.len()];
    let bytes_read = buffer.read(&mut read_data).unwrap();
    
    assert_eq!(bytes_read, data.len());
    assert_eq!(read_data, data);
    assert_eq!(buffer.available_data(), 0);
    assert_eq!(buffer.remaining_capacity(), 1024);
}

#[test]
fn test_buffer_read_from() {
    let mut buffer = Buffer::new(1024);
    let data = b"Hello, World!";
    let mut cursor = Cursor::new(data);
    
    let bytes_read = buffer.read_from(&mut cursor).unwrap();
    assert_eq!(bytes_read, data.len());
    assert_eq!(buffer.available_data(), data.len());
    assert_eq!(buffer.slice(), data);
}

#[test]
fn test_buffer_write_to() {
    let mut buffer = Buffer::new(1024);
    let data = b"Hello, World!";
    buffer.write(data).unwrap();
    
    let mut output = Vec::new();
    let bytes_written = buffer.write_to(&mut output).unwrap();
    
    assert_eq!(bytes_written, data.len());
    assert_eq!(output, data);
    assert_eq!(buffer.available_data(), 0);
}

#[test]
fn test_buffer_auto_resize() {
    let mut buffer = Buffer::new(16);
    let data = vec![0; 32];
    
    let bytes_written = buffer.write(&data).unwrap();
    assert_eq!(bytes_written, data.len());
    assert!(buffer.capacity() >= 32);
    assert_eq!(buffer.available_data(), 32);
}

#[test]
fn test_buffer_compaction() {
    let mut buffer = Buffer::new(16);
    
    // Write some data
    buffer.write(b"0123456789").unwrap();
    assert_eq!(buffer.available_data(), 10);
    
    // Read part of it
    let mut read_data = vec![0; 5];
    buffer.read(&mut read_data).unwrap();
    assert_eq!(buffer.available_data(), 5);
    
    // Write more data - this should trigger compaction
    buffer.write(b"ABCDEFGHIJ").unwrap();
    assert_eq!(buffer.available_data(), 15);
    assert_eq!(buffer.slice(), b"56789ABCDEFGHIJ");
}

#[test]
fn test_buffer_reset() {
    let mut buffer = Buffer::new(1024);
    buffer.write(b"Hello, World!").unwrap();
    
    buffer.reset();
    assert_eq!(buffer.available_data(), 0);
    assert_eq!(buffer.remaining_capacity(), 1024);
}