//! Module implementing panic handler that calls the host's `abort` method.

use crate::ffi::string::{AscStr, AscString};
use std::panic;

/// Sets the panic hook to use the host provided `abort` call.
pub fn set_panic_hook() {
    panic::set_hook(Box::new(|info| {
        let (message, location) = if let Some(message) = info.payload().downcast_ref::<&str>() {
            (AscString::new(message), info.location())
        } else if let Some(message) = info.payload().downcast_ref::<String>() {
            (AscString::new(message), info.location())
        } else {
            (AscString::new(info.to_string()), None)
        };
        let (file, line, column) = if let Some(location) = location {
            (
                Some(AscString::new(location.file())),
                location.line(),
                location.column(),
            )
        } else {
            (None, 0, 0)
        };

        let file = file.as_ref().map(|f| f.as_asc_str());
        unsafe {
            abort(&*message, file, line, column);
        }
    }));
}

#[link(wasm_import_module = "env")]
extern "C" {
    #[link_name = "abort"]
    fn abort(message: &AscStr, file: Option<&AscStr>, line: u32, column: u32) -> !;
}
