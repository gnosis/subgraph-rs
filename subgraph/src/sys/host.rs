//! The Graph host import function bindings.

#![allow(non_snake_case)]

use super::BigInt;
use crate::ffi::string::AscStr;

#[link(wasm_import_module = "env")]
extern "C" {
    #[link_name = "abort"]
    pub fn abort(message: &AscStr, file: Option<&AscStr>, line: u32, column: u32) -> !;
}

pub mod bigInt {
    use super::*;

    #[link(wasm_import_module = "index")]
    extern "C" {
        #[link_name = "bigInt.plus"]
        pub fn plus<'host>(x: BigInt, y: BigInt) -> BigInt<'host>;
    }
}

pub mod log {
    use super::*;

    #[link(wasm_import_module = "index")]
    extern "C" {
        #[link_name = "log.log"]
        pub fn log(level: u32, message: &AscStr);
    }
}

pub mod typeConversion {
    use super::*;

    #[link(wasm_import_module = "index")]
    extern "C" {
        #[link_name = "typeConversion.bigIntToString"]
        pub fn bigIntToString(x: BigInt) -> &AscStr;
    }
}
