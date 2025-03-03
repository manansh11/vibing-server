use crate::error::{ServerError, ServerResult};
use std::io::{self, Read, Write};
use std::ptr;

/// A resizable buffer with efficient memory management
pub struct Buffer {
    data: Vec<u8>,
    read_pos: usize,
    write_pos: usize,
}

impl Buffer {
    /// Create a new buffer with the specified capacity
    pub fn new(capacity: usize) -> Self {
        Self {
            data: vec![0; capacity],
            read_pos: 0,
            write_pos: 0,
        }
    }
    
    /// Read data from a reader into the buffer
    pub fn read_from<R: Read>(&mut self, reader: &mut R) -> io::Result<usize> {
        // Ensure we have space
        self.ensure_capacity(1024);
        
        // Read directly into the buffer at the write position
        let bytes_read = reader.read(&mut self.data[self.write_pos..])?;
        self.write_pos += bytes_read;
        
        Ok(bytes_read)
    }
    
    /// Write data from the buffer to a writer
    pub fn write_to<W: Write>(&mut self, writer: &mut W) -> io::Result<usize> {
        let available = self.available_data();
        if available == 0 {
            return Ok(0);
        }
        
        let bytes_written = writer.write(&self.data[self.read_pos..self.write_pos])?;
        self.read_pos += bytes_written;
        
        // If we've read everything, reset positions
        if self.read_pos == self.write_pos {
            self.reset();
        }
        
        Ok(bytes_written)
    }
    
    /// Ensure the buffer has at least the specified additional capacity
    pub fn ensure_capacity(&mut self, additional: usize) {
        let available_capacity = self.data.len() - self.write_pos;
        
        if available_capacity >= additional {
            return;
        }
        
        // Compact the buffer if possible
        if self.read_pos > 0 {
            let len = self.write_pos - self.read_pos;
            unsafe {
                ptr::copy(
                    self.data.as_ptr().add(self.read_pos),
                    self.data.as_mut_ptr(),
                    len,
                );
            }
            self.write_pos = len;
            self.read_pos = 0;
        }
        
        // Resize if still needed
        let available_after_compact = self.data.len() - self.write_pos;
        if available_after_compact < additional {
            let new_capacity = (self.data.len() + additional).max(self.data.len() * 2);
            self.data.resize(new_capacity, 0);
        }
    }
    
    /// Reset the buffer, clearing all data
    pub fn reset(&mut self) {
        self.read_pos = 0;
        self.write_pos = 0;
    }
    
    /// Get the amount of data available to read
    pub fn available_data(&self) -> usize {
        self.write_pos - self.read_pos
    }
    
    /// Get the remaining capacity in the buffer
    pub fn remaining_capacity(&self) -> usize {
        self.data.len() - self.write_pos
    }
    
    /// Write a slice of data to the buffer
    pub fn write(&mut self, data: &[u8]) -> ServerResult<usize> {
        self.ensure_capacity(data.len());
        
        let to_copy = data.len().min(self.remaining_capacity());
        self.data[self.write_pos..self.write_pos + to_copy].copy_from_slice(&data[..to_copy]);
        self.write_pos += to_copy;
        
        Ok(to_copy)
    }
    
    /// Read a slice of data from the buffer
    pub fn read(&mut self, data: &mut [u8]) -> ServerResult<usize> {
        let available = self.available_data();
        if available == 0 {
            return Ok(0);
        }
        
        let to_copy = data.len().min(available);
        data[..to_copy].copy_from_slice(&self.data[self.read_pos..self.read_pos + to_copy]);
        self.read_pos += to_copy;
        
        // If we've read everything, reset positions
        if self.read_pos == self.write_pos {
            self.reset();
        }
        
        Ok(to_copy)
    }
    
    /// Get a slice of the buffer's data
    pub fn slice(&self) -> &[u8] {
        &self.data[self.read_pos..self.write_pos]
    }
    
    /// Get a mutable slice of the buffer's data
    pub fn slice_mut(&mut self) -> &mut [u8] {
        &mut self.data[self.read_pos..self.write_pos]
    }
    
    /// Get the total capacity of the buffer
    pub fn capacity(&self) -> usize {
        self.data.len()
    }
    
    /// Advance the read position by the specified amount
    pub fn advance_read(&mut self, amount: usize) -> ServerResult<()> {
        let available = self.available_data();
        if amount > available {
            return Err(ServerError::Buffer(format!("Cannot advance read position beyond write position ({} > {})", amount, available)));
        }
        
        self.read_pos += amount;
        
        // If we've read everything, reset positions
        if self.read_pos == self.write_pos {
            self.reset();
        }
        
        Ok(())
    }
}