//! Subgraph arbitrary precision integer implementation.

/*
use crate::ffi::{array::AscTypedArray, string::AscStr};

/// A arbitrary precision big integer. This uses the host big integer
/// implementation through the provided import functions.
pub struct BigInt(AscTypedArray<u8>);

#[link(wasm_import_module = "index")]
extern "C" {
    #[link_name = "bigInt.plus"]
    fn plus(x: BigInt, message: BigInt) -> BigInt;

    #[link_name = "typeConversion.bigIntToString"]
    fn bigIntToString(x: BigInt) -> &AscStr;
}
*/
