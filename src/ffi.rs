//! Module containing FFI utilities for mapping Rust/C ABI to the AssemblyScript
//! ABI.
//!
//! # `'host` Lifetime
//!
//! Special consideration should be taken with dealing with FFI functions that
//! return values with a `'host` lifetime. The Graph host allocates memory to
//! fill with return data in various functions. As of the this writing, these
//! allocations are done in a host controlled arena allocator memory region and
//! does not seem to provide a way to free this memory. Furthermore, additional
//! allocations may cause the host to re-allocate a new region if it runs out of
//! space in the arena allocator's memory region.
//!
//! # Safety
//!
//! Data from host functions that return references or pointers must be cloned
//! into Rust-owned memory before any futher host allocations occur.

pub mod array;
mod buffer;
pub mod string;
