//! AssemblyScript array buffer and typed array implementations.

use crate::ffi::buffer::{AscBuf, AscBuffer};

/// An owned AssemblyScript array buffer.
pub type AscArrayBuffer = AscBuffer<u8, u64>;

impl AscArrayBuffer {
    /// Returns the the array buffer as a slice of bytes.
    pub fn as_bytes(&self) -> &[u8] {
        self.as_slice()
    }
}

/// A `u8` typed array that slices an array buffer.
#[repr(C)]
pub struct AscUint8Array<'a> {
    buffer: &'a AscBuf<u8, u64>,
    offset: usize,
    len: usize,
}

impl<'a> AscUint8Array<'a> {
    /// Creates a typed array view over the entire specifed array buffer.
    pub fn new(buffer: &'a AscBuf<u8, u64>) -> Self {
        Self {
            buffer,
            offset: 0,
            len: buffer.len(),
        }
    }

    /// Returns the `u8` typed array as a Rust slice.
    pub fn as_bytes(&self) -> &[u8] {
        &self.buffer.as_slice()[self.offset..(self.offset + self.len)]
    }

    /// Creates an owned AssemblyScript array buffer from the sliced bytes.
    pub fn to_array_buffer(&self) -> Box<AscArrayBuffer> {
        AscArrayBuffer::new(self.as_bytes())
    }
}
