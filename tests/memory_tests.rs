use high_performance_server::memory::{MemoryManager, MemoryPool};

#[test]
fn test_memory_pool_creation() {
    let pool = MemoryPool::new(64, 10);
    assert_eq!(pool.capacity(), 10);
    assert_eq!(pool.in_use(), 0);
    assert_eq!(pool.size_class(), 64);
}

#[test]
fn test_memory_pool_allocate_deallocate() {
    let mut pool = MemoryPool::new(64, 10);
    
    // Allocate a block
    let ptr = pool.allocate().unwrap();
    assert_eq!(pool.in_use(), 1);
    
    // Write to the memory
    unsafe {
        *ptr.as_ptr() = 42;
    }
    
    // Deallocate the block
    pool.deallocate(ptr).unwrap();
    assert_eq!(pool.in_use(), 0);
}

#[test]
fn test_memory_pool_auto_resize() {
    let mut pool = MemoryPool::new(64, 2);
    
    // Allocate all initial blocks
    let ptr1 = pool.allocate().unwrap();
    let ptr2 = pool.allocate().unwrap();
    assert_eq!(pool.in_use(), 2);
    
    // Allocate one more - should trigger resize
    let ptr3 = pool.allocate().unwrap();
    assert!(pool.capacity() > 2);
    assert_eq!(pool.in_use(), 3);
    
    // Clean up
    pool.deallocate(ptr1).unwrap();
    pool.deallocate(ptr2).unwrap();
    pool.deallocate(ptr3).unwrap();
}

#[test]
fn test_memory_pool_many_allocations() {
    let mut pool = MemoryPool::new(32, 5);
    
    const NUM_ALLOCS: usize = 100;
    let mut ptrs = Vec::with_capacity(NUM_ALLOCS);
    
    // Allocate many blocks
    for _ in 0..NUM_ALLOCS {
        ptrs.push(pool.allocate().unwrap());
    }
    
    assert_eq!(pool.in_use(), NUM_ALLOCS);
    assert!(pool.capacity() >= NUM_ALLOCS);
    
    // Clean up
    for ptr in ptrs {
        pool.deallocate(ptr).unwrap();
    }
    
    assert_eq!(pool.in_use(), 0);
}

#[test]
fn test_memory_manager() {
    let manager = MemoryManager::new();
    
    // Allocate different sized blocks
    let mut handle1 = manager.allocate(32).unwrap();
    let mut handle2 = manager.allocate(64).unwrap();
    let mut handle3 = manager.allocate(128).unwrap();
    
    // Write to the memory
    handle1.as_slice_mut()[0] = 1;
    handle2.as_slice_mut()[0] = 2;
    handle3.as_slice_mut()[0] = 3;
    
    // Read from the memory
    assert_eq!(handle1.as_slice()[0], 1);
    assert_eq!(handle2.as_slice()[0], 2);
    assert_eq!(handle3.as_slice()[0], 3);
    
    // Check sizes
    assert!(handle1.size() >= 32);
    assert!(handle2.size() >= 64);
    assert!(handle3.size() >= 128);
}

#[test]
fn test_memory_manager_many_allocations() {
    let manager = MemoryManager::new();
    
    const NUM_ALLOCS: usize = 1000;
    let mut handles = Vec::with_capacity(NUM_ALLOCS);
    
    // Allocate many blocks of different sizes
    for i in 0..NUM_ALLOCS {
        let size = match i % 4 {
            0 => 32,
            1 => 64,
            2 => 128,
            _ => 256,
        };
        
        handles.push(manager.allocate(size).unwrap());
    }
    
    // Write to each block
    for (i, handle) in handles.iter_mut().enumerate() {
        handle.as_slice_mut()[0] = (i % 255) as u8;
    }
    
    // Read from each block
    for (i, handle) in handles.iter().enumerate() {
        assert_eq!(handle.as_slice()[0], (i % 255) as u8);
    }
}

#[test]
fn test_create_buffer() {
    let manager = MemoryManager::new();
    let mut buffer = manager.create_buffer(1024).unwrap();
    
    assert!(buffer.size() >= 1024);
    
    // Test writing to the buffer
    let data = &mut buffer.as_slice_mut();
    for i in 0..10 {
        data[i] = i as u8;
    }
    
    // Test reading from the buffer
    let data = buffer.as_slice();
    for i in 0..10 {
        assert_eq!(data[i], i as u8);
    }
}