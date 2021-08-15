//! Subgraph arbitrary precision integer implementation.

use crate::{ffi::array::AscArrayBuffer, sys};
use std::fmt::{self, Debug, Display, Formatter};

/// A arbitrary precision big integer. This uses the host big integer
/// implementation through the provided import functions.
///
/// `BitInt` is represented on the host as its little-endian bytes.
pub struct BigInt {
    inner: Box<AscArrayBuffer>,
}

impl BigInt {
    /// Creates a `BigInt` instance from unsigned little endian bytes.
    pub fn from_unsigned_bytes_le(bytes: impl AsRef<[u8]>) -> Self {
        let bytes = bytes.as_ref();
        if matches!(bytes.last(), Some(byte) if byte & 0x80 != 0) {
            // NOTE: We need to append an extra `0`-byte so that the value isn't
            // treated as negative.
            let mut corrected_bytes = Vec::with_capacity(bytes.len() + 1);
            corrected_bytes.extend_from_slice(bytes);
            corrected_bytes.push(0);
            Self::from_signed_bytes_le(&corrected_bytes)
        } else {
            Self::from_signed_bytes_le(bytes)
        }
    }

    /// Creates a `BigInt` instance from signed little endian bytes.
    pub fn from_signed_bytes_le(bytes: impl AsRef<[u8]>) -> Self {
        Self {
            inner: AscArrayBuffer::new(bytes.as_ref()),
        }
    }

    /// Add the specified `BigInt` to `self`, returning the result.
    pub fn add(&self, rhs: &Self) -> Self {
        let x = self.as_host();
        let y = rhs.as_host();

        // SAFETY: The host allocation gets cloned to an owned array buffer.
        let inner = unsafe { sys::bigInt::plus(x, y).to_array_buffer() };

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
            match last_byte & 0x80 {
                0 => 1,
                _ => -1,
            }
        }
    }

    fn as_host(&self) -> sys::BigInt<'_> {
        sys::BigInt::new(&self.inner)
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
            let asc_str = unsafe { sys::typeConversion::bigIntToString(x) };
            asc_str
                .to_string()
                .expect("integer strings are always valid UTF-16")
        };

        f.pad_integral(self.signum() >= 0, "", &s)
    }
}

macro_rules! from_primitive {
    ($(
        $m:ident : $($t:ty),* ;
    )*) => {$($(
        impl From<$t> for BigInt {
            fn from(x: $t) -> Self {
                Self::$m(&x.to_le_bytes())
            }
        }
    )*)*};
}

from_primitive! {
    from_signed_bytes_le: i8, i16, i32, i64, i128, isize;
    from_unsigned_bytes_le: u8, u16, u32, u64, u128, usize;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn primitive_conversion() {
        let x = BigInt::from(42u32);
        assert_eq!(x.inner.as_bytes(), [42, 0, 0, 0]);

        let x = BigInt::from(u32::MAX);
        assert_eq!(x.inner.as_bytes(), [0xff, 0xff, 0xff, 0xff, 0]);

        let x = BigInt::from(-1i32);
        assert_eq!(x.inner.as_bytes(), [0xff, 0xff, 0xff, 0xff]);
    }

    #[test]
    fn signum() {
        assert_eq!(BigInt::from(0).signum(), 0);
        assert_eq!(BigInt::from(42).signum(), 1);
        assert_eq!(BigInt::from(u32::MAX).signum(), 1);
        assert_eq!(BigInt::from(-1337).signum(), -1);
        assert_eq!(BigInt::from(i32::MIN).signum(), -1);
    }

    // TODO(nlordell): This is a useful test, but requires mocking the imported
    // host functions (specifically `bigIntToString`).
    /*
    #[test]
    fn to_string() {
        let pos = BigInt::from(42i32);
        let neg = BigInt::from(-1337i32);

        assert_eq!(format!("{}", pos), "42");
        assert_eq!(format!("{:^8}", pos), "^^^^^^42");
        assert_eq!(format!("{:-.8}", pos), "42......");

        assert_eq!(format!("{}", neg), "-1337");
        assert_eq!(format!("{:^8}", neg), "^^^-1337");
        assert_eq!(format!("{:-.8}", neg), "-1337...");
    }
    */
}
