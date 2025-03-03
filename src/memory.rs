use crate::error::{ServerError, ServerResult};
use std::ptr::{NonNull};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

/// Block of memory in a memory pool
struct MemoryBlock {
    ptr: NonNull<u8>,
    size: usize,
    in_use: bool,
}

/// A memory pool for efficient allocation and reuse of fixed-size memory blocks
pub struct MemoryPool {
    // Chunks of memory that the pool owns
    chunks: Vec<Vec<u8>>,
    
    // Index of available blocks within the chunks
    blocks: Vec<MemoryBlock>,
    
    // Size of each block
    block_size: usize,
    
    // Total capacity of the pool
    capacity: usize,
    
    // Number of blocks in use
    in_use: AtomicUsize,
    
    // Size class of this pool
    size_class: usize,
}

impl MemoryPool {
    /// Create a new memory pool with blocks of the specified size
    pub fn new(block_size: usize, initial_blocks: usize) -> Self {
        let mut pool = Self {
            chunks: Vec::new(),
            blocks: Vec::with_capacity(initial_blocks),
            block_size,
            capacity: 0,
            in_use: AtomicUsize::new(0),
            size_class: block_size,
        };
        
        // Allocate initial memory
        pool.grow(initial_blocks);
        
        pool
    }
    
    /// Grow the pool by adding more blocks
    fn grow(&mut self, additional_blocks: usize) {
        let chunk_size = self.block_size * additional_blocks;
        let mut chunk = Vec::with_capacity(chunk_size);
        chunk.resize(chunk_size, 0);
        
        // Track blocks in this chunk
        let base_ptr = chunk.as_mut_ptr();
        for i in 0..additional_blocks {
            let offset = i * self.block_size;
            let ptr = unsafe { NonNull::new_unchecked(base_ptr.add(offset)) };
            
            self.blocks.push(MemoryBlock {
                ptr,
                size: self.block_size,
                in_use: false,
            });
        }
        
        self.capacity += additional_blocks;
        self.chunks.push(chunk);
    }
    
    /// Allocate a block of memory from the pool
    pub fn allocate(&mut self) -> ServerResult<NonNull<u8>> {
        // Find an available block
        for block in &mut self.blocks {
            if !block.in_use {
                block.in_use = true;
                self.in_use.fetch_add(1, Ordering::Relaxed);
                return Ok(block.ptr);
            }
        }
        
        // If no blocks are available, grow the pool
        let additional_blocks = (self.capacity / 2).max(1);
        self.grow(additional_blocks);
        
        // Now there should be at least one free block
        for block in &mut self.blocks.iter_mut().skip(self.capacity - additional_blocks) {
            if !block.in_use {
                block.in_use = true;
                self.in_use.fetch_add(1, Ordering::Relaxed);
                return Ok(block.ptr);
            }
        }
        
        // This should never happen, but just in case
        Err(ServerError::Memory("Failed to allocate memory block".to_string()))
    }
    
    /// Deallocate a block of memory back to the pool
    pub fn deallocate(&mut self, ptr: NonNull<u8>) -> ServerResult<()> {
        // Find the block
        for block in &mut self.blocks {
            if block.ptr.as_ptr() == ptr.as_ptr() && block.in_use {
                block.in_use = false;
                self.in_use.fetch_sub(1, Ordering::Relaxed);
                return Ok(());
            }
        }
        
        Err(ServerError::Memory("Block not found in pool".to_string()))
    }
    
    /// Resize the pool to handle a different number of blocks
    pub fn resize(&mut self, new_capacity: usize) -> ServerResult<()> {
        if new_capacity < self.in_use.load(Ordering::Relaxed) {
            return Err(ServerError::Memory(
                "Cannot resize pool smaller than number of blocks in use".to_string(),
            ));
        }
        
        if new_capacity > self.capacity {
            self.grow(new_capacity - self.capacity);
        }
        
        Ok(())
    }
    
    /// Get the current total capacity of the pool
    pub fn capacity(&self) -> usize {
        self.capacity
    }
    
    /// Get the number of blocks currently in use
    pub fn in_use(&self) -> usize {
        self.in_use.load(Ordering::Relaxed)
    }
    
    /// Get the size class (block size) of this pool
    pub fn size_class(&self) -> usize {
        self.size_class
    }
}

/// A thread-safe memory allocator that manages multiple pools
pub struct MemoryAllocator {
    // Pools for different size classes
    pools: Mutex<Vec<MemoryPool>>,
    
    // Common size classes (powers of 2)
    size_classes: Vec<usize>,
}

impl MemoryAllocator {
    /// Create a new memory allocator
    pub fn new() -> Self {
        // Create size classes as powers of 2
        let mut size_classes = Vec::new();
        let mut size = 16; // Start with 16 bytes
        while size <= 8192 { // Up to 8KB
            size_classes.push(size);
            size *= 2;
        }
        
        // Create pools for each size class
        let mut pools = Vec::with_capacity(size_classes.len());
        for &size in &size_classes {
            pools.push(MemoryPool::new(size, 16)); // 16 initial blocks per pool
        }
        
        Self {
            pools: Mutex::new(pools),
            size_classes,
        }
    }
    
    /// Find the appropriate size class for a given size
    fn find_size_class(&self, size: usize) -> usize {
        for &class in &self.size_classes {
            if size <= class {
                return class;
            }
        }
        
        // If larger than any size class, use the largest
        *self.size_classes.last().unwrap()
    }
    
    /// Allocate memory of the specified size
    pub fn allocate(&self, size: usize) -> ServerResult<(NonNull<u8>, usize)> {
        let size_class = self.find_size_class(size);
        let size_class_index = self.size_classes.iter().position(|&s| s == size_class).unwrap();
        
        let mut pools = self.pools.lock().unwrap();
        let pool = &mut pools[size_class_index];
        
        let ptr = pool.allocate()?;
        
        Ok((ptr, size_class))
    }
    
    /// Deallocate memory previously allocated by this allocator
    pub fn deallocate(&self, ptr: NonNull<u8>, size_class: usize) -> ServerResult<()> {
        let size_class_index = self.size_classes.iter().position(|&s| s == size_class)
            .ok_or_else(|| ServerError::Memory("Invalid size class".to_string()))?;
        
        let mut pools = self.pools.lock().unwrap();
        let pool = &mut pools[size_class_index];
        
        pool.deallocate(ptr)
    }
}

/// A reference-counted wrapper for memory allocation
pub struct MemoryManager {
    allocator: Arc<MemoryAllocator>,
}

impl MemoryManager {
    /// Create a new memory manager
    pub fn new() -> Self {
        Self {
            allocator: Arc::new(MemoryAllocator::new()),
        }
    }
    
    /// Allocate memory of the specified size
    pub fn allocate(&self, size: usize) -> ServerResult<MemoryHandle> {
        let (ptr, size_class) = self.allocator.allocate(size)?;
        
        Ok(MemoryHandle {
            ptr,
            size_class,
            allocator: self.allocator.clone(),
        })
    }
    
    /// Create a memory handle for a buffer of the specified size
    pub fn create_buffer(&self, size: usize) -> ServerResult<MemoryHandle> {
        self.allocate(size)
    }
}

impl Default for MemoryManager {
    fn default() -> Self {
        Self::new()
    }
}

/// A handle to a block of memory allocated by the memory manager
pub struct MemoryHandle {
    ptr: NonNull<u8>,
    size_class: usize,
    allocator: Arc<MemoryAllocator>,
}

impl MemoryHandle {
    /// Get a reference to the memory as a slice
    pub fn as_slice(&self) -> &[u8] {
        unsafe { std::slice::from_raw_parts(self.ptr.as_ptr(), self.size_class) }
    }
    
    /// Get a mutable reference to the memory as a slice
    pub fn as_slice_mut(&mut self) -> &mut [u8] {
        unsafe { std::slice::from_raw_parts_mut(self.ptr.as_ptr(), self.size_class) }
    }
    
    /// Get the size of this memory block
    pub fn size(&self) -> usize {
        self.size_class
    }
}

impl Drop for MemoryHandle {
    fn drop(&mut self) {
        // Deallocate the memory when the handle is dropped
        let _ = self.allocator.deallocate(self.ptr, self.size_class);
    }
}