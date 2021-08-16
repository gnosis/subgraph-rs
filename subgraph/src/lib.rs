//! [The Graph](https://thegraph.com/docs/introduction) subgraph bindings for
//! Rust 🦀

mod abort;
mod ffi;
mod logger;
mod num;
mod sys;

pub use self::num::bigint::BigInt;
pub use log;

/// Module containing required Wasm exports.
#[cfg(target_arch = "wasm32")]
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

    #[no_mangle]
    #[link_section = "apiVersion"]
    pub static API_VERSION: [u8; 5] = *b"0.0.4";
}

/// Unused exports when not targetting for The Graph host. This unused method
/// declaration ensures that certain methods that don't have a public API don't
/// cause `unused` class linter errors in that case.
#[cfg(not(target_arch = "wasm32"))]
fn unused_exports() {
    #![allow(unused)]
    let _ = abort::set_panic_hook;
    let _ = logger::init;
}
