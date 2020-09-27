//! Subgraph arbitrary precision integer implementation.

use crate::ffi::{
    array::{AscArrayBuffer, AscUint8Array},
    string::AscStr,
};
use std::{
    fmt::{self, Debug, Display, Formatter},
    ops::Add,
};

/// A arbitrary precision big integer. This uses the host big integer
/// implementation through the provided import functions.
///
/// `BitInt` is represented on the host as its little-endian bytes.
pub struct BigInt {
    inner: AscArrayBuffer,
}

impl BigInt {
    /// Add the specified `BigInt` to `self`, returning the result.
    pub fn add(&self, rhs: &Self) -> Self {
        let x = self.as_host();
        let y = rhs.as_host();

        // SAFETY: The host allocation gets cloned to an owned array buffer.
        let inner = unsafe { plus(x, y).to_array_buffer() };

        Self { inner }
    }

    /// Returns a number representing the sign of `self`.
    /// - `0` if the number is 0
    /// - `1` if the number is positive
    /// - `-1` if the number is negative
    pub fn signum(&self) -> i32 {
        let bytes = self.inner.as_bytes();

        // NOTE: In LE, the most significant bit, which contains the sign
        // information is the last byte.
        if bytes.iter().all(|b| *b == 0) {
            0
        } else {
            let last_byte = bytes.last().copied().unwrap_or(0);
            (last_byte as i8).signum() as _
        }
    }

    fn as_host(&self) -> HostBigInt<'_> {
        HostBigInt::new(&self.inner)
    }
}

impl Debug for BigInt {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        Display::fmt(self, f)
    }
}

impl Display for BigInt {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        let x = self.as_host();
        let s = {
            let asc_str = unsafe { bigIntToString(x) };
            asc_str
                .to_string()
                .expect("integer strings are always valid UTF-16")
        };

        f.pad_integral(self.signum() >= 0, "", &s)
    }
}

impl<'a> Add for &'a BigInt {
    type Output = BigInt;

    fn add(self, rhs: Self) -> Self::Output {
        self.add(rhs)
    }
}

macro_rules! from_primitive {
    ($($t:ty),* $(,)?) => {$(
        impl From<$t> for BigInt {
            fn from(x: $t) -> Self {
                Self {
                    inner: AscArrayBuffer::new(x.to_le_bytes()),
                }
            }
        }
    )*};
}

from_primitive! {
    i8, i16, i32, i64, i128, isize,
    u8, u16, u32, u64, u128, usize,
}

/// The host `BigInt` type.
type HostBigInt<'a> = AscUint8Array<'a>;

#[link(wasm_import_module = "index")]
extern "C" {
    #[link_name = "bigInt.plus"]
    fn plus<'host>(x: HostBigInt, y: HostBigInt) -> HostBigInt<'host>;

    #[link_name = "typeConversion.bigIntToString"]
    fn bigIntToString(x: HostBigInt) -> &AscStr;
}
