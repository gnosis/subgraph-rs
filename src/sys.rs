//! Host import function bindings.

#[cfg(target_arch = "wasm32")]
#[path = "sys/host.rs"]
mod bindings;

#[cfg(not(target_arch = "wasm32"))]
#[path = "sys/mock.rs"]
mod bindings;

pub use self::bindings::*;
use crate::ffi::array::AscUint8Array;

/// The host `BigInt` type.
pub type BigInt<'a> = AscUint8Array<'a>;
