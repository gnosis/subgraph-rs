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
        mem,
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
        unsafe { alloc::alloc(Layout::from_size_align_unchecked(ALIGN, size)) }
    }
}
