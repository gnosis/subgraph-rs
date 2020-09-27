//! AssemblyScript array buffer and typed array implementations.

use crate::ffi::buffer::{AscBuf, AscBuffer};
use std::{
    marker::PhantomData,
    ops::{Bound, RangeBounds},
};

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

    /// Returns a new sub-slice of the `u8` typed array with the specified
    /// range.
    ///
    /// # Panics
    ///
    /// Panics if the slice specifed by `range` is out of bounds of the typed
    /// array.
    pub fn slice(&self, range: impl RangeBounds<usize>) -> AscUint8Array<'_> {
        let offset = self.offset
            + match range.start_bound() {
                Bound::Included(value) => *value,
                Bound::Excluded(value) => value.saturating_add(1),
                Bound::Unbounded => 0,
            };
        let len = {
            let end = match range.end_bound() {
                Bound::Included(value) => value.saturating_add(1),
                Bound::Excluded(value) => *value,
                Bound::Unbounded => 0,
            };
            end.checked_sub(offset)
                .unwrap_or_else(|| panic!("slice index starts at {} but ends at {}", offset, end))
        };

        assert!(self.len >= offset + len);
        Self {
            buffer: &self.buffer,
            offset,
            len,
        }
    }

    /// Returns the `u8` typed array as a Rust slice.
    pub fn as_bytes(&self) -> &[u8] {
        &self.buffer.as_slice()[self.offset..(self.offset + self.len)]
    }

    /// Creates an owned AssemblyScript array buffer from the sliced bytes.
    pub fn to_array_buffer(&self) -> AscArrayBuffer {
        AscArrayBuffer::new(self.as_bytes())
    }
}
