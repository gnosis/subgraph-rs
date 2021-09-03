//! Mock host import function bindings.

#![allow(non_snake_case)]

use super::BigInt;
use crate::ffi::string::AscStr;

pub unsafe fn abort(_: &AscStr, _: Option<&AscStr>, _: u32, _: u32) -> ! {
    unreachable!("mocked abort host method called");
}

pub mod bigInt {
    use super::*;

    pub unsafe fn plus(_x: &BigInt, _y: &BigInt) -> *mut BigInt<'static> {
        todo!()
    }
}

pub mod log {
    use super::*;

    pub unsafe fn log(_: u32, _: &AscStr) {
        unreachable!("mocked log host method called");
    }
}

pub mod typeConversion {
    use super::*;

    pub unsafe fn bigIntToString(_x: BigInt) -> *mut AscStr {
        todo!()
    }
}
