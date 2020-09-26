//! AssemblyScript string implementation.

use crate::ffi::buffer::{AscBuf, AscBuffer};
use std::{
    fmt::{self, Debug, Formatter},
    ops::Deref,
    string::FromUtf16Error,
};

/// A borrowed AssemblyScript string.
///
/// Dynamically sized types are not safe over FFI boundries, so it is important
/// to only use `AscStr` (and not `AscString`) as exported and imported function
/// parameter types.
#[repr(transparent)]
pub struct AscStr {
    inner: AscBuf<u16>,
}

impl AscStr {
    /// Converts the AssemblyScript string into a Rust `String`.
    #[allow(unused)] // TODO(nlordell): Remove once it is used.
    pub fn to_string(&self) -> Result<String, FromUtf16Error> {
        String::from_utf16(&self.inner.as_slice())
    }

    /// Converts the AssemblyScript string into a Rust `String`, replacing
    /// invalid data with the replacement character (`U+FFFD`).
    pub fn to_string_lossy(&self) -> String {
        String::from_utf16_lossy(&self.inner.as_slice())
    }
}

impl Debug for AscStr {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        Debug::fmt(&self.to_string_lossy(), f)
    }
}

/// An AssemblyScript string.
pub struct AscString {
    inner: Box<AscBuffer<u16>>,
}

impl AscString {
    /// Creates a new AssemblyScript string from a Rust string slice.
    pub fn new(s: impl AsRef<str>) -> Self {
        let s = s.as_ref();

        let code_points = {
            let mut buffer = Vec::with_capacity(s.len());
            buffer.extend(s.encode_utf16());
            buffer
        };
        let inner = AscBuffer::new(&code_points);

        AscString { inner }
    }

    /// Returns a reference to a borrowed AssemblyScript string.
    pub fn as_asc_str(&self) -> &AscStr {
        // SAFETY: `AscStr` has a `transparent` representation and so has an
        // identical memory representation to a `AscBuf<u16>`.
        unsafe { &*self.inner.as_buf_ptr().cast() }
    }
}

impl Deref for AscString {
    type Target = AscStr;

    fn deref(&self) -> &Self::Target {
        self.as_asc_str()
    }
}

impl Debug for AscString {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        Debug::fmt(self.as_asc_str(), f)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::alloc::Layout;

    #[test]
    fn can_round_trip_str() {
        let message = "Hello 🦀";
        assert_eq!(AscString::new(message).to_string().unwrap(), message);
    }

    #[test]
    fn string_layout() {
        let string = AscString::new("0123456");
        assert_eq!(string.inner.as_slice().len(), 7);
        assert_eq!(
            Layout::for_value(&*string.inner),
            Layout::new::<(usize, [u16; 7])>().pad_to_align(),
        );
    }

    #[test]
    fn str_layout() {
        let string = AscString::new("0123456");
        assert_eq!(
            Layout::for_value(string.as_asc_str()),
            Layout::new::<(usize, [u16; 0])>().pad_to_align(),
        );
    }
}
