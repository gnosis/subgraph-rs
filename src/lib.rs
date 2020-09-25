mod abort;
mod ffi;
mod logger;

pub use log;

/// Module containing required Wasm exports.
#[doc(hidden)]
pub mod exports {
    use crate::{abort, logger};

    /// The Wasm start function. This gets set as the module's start function
    /// during post-processing of the Wasm blob, since there is currently no
    /// way to specify it in Rust at the moment.
    #[export_name = "__subgraph_start"]
    pub extern "C" fn start() {
        abort::set_panic_hook();
        logger::init();
    }
}
