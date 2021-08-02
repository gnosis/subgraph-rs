//! [The Graph](https://thegraph.com/docs/introduction) subgraph bindings for
//! Rust 🦀

mod abort;
mod ffi;
mod logger;

pub use log;

/// Module containing required Wasm exports.
#[doc(hidden)]
pub mod exports {
    use crate::{abort, logger};
    use std::{
        alloc::{self, Layout},
        mem, ptr,
    };

    /// The Wasm start function. This gets set as the module's start function
    /// during post-processing of the Wasm blob, since there is currently no
    /// way to specify it in Rust at the moment.
    #[export_name = "__subgraph_start"]
    pub extern "C" fn start() {
        abort::set_panic_hook();
        logger::init();
    }

    /// A hook into the Rust memory allocation function so that the host may
    /// allocate space for data to be sent to the mapping handlers.
    #[export_name = "memory.allocate"]
    pub extern "C" fn alloc(size: usize) -> *mut u8 {
        // NOTE: Use the maximum wasm32 alignment by default.
        const ALIGN: usize = mem::size_of::<u64>();

        let layout = match Layout::from_size_align(ALIGN, size) {
            Ok(value) => value,
            Err(_) => {
                // NOTE: Since `ALIGN` is guaranteed to be valid, this can only
                // happen if `size` overflows when padding to `ALIGN`. Return
                // null to signal that the allocation failed.
                return ptr::null_mut();
            }
        };

        unsafe { alloc::alloc(layout) }
    }
}
