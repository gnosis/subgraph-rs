//! AssemblyScript string implementation.

use std::{
    alloc::{self, Layout, LayoutErr},
    fmt::{self, Debug, Formatter},
    mem,
    ops::Deref,
    slice,
    string::FromUtf16Error,
};

/// Internal representation of an AssemblyScript string.
///
/// AssemblyScript strings are length prefixed utf-16 strings. That is, they
/// are laid out in memory starting with a length `l` (usize is 32-bits in the
/// `wasm32-*` targets) followd by `l` utf-16 `u16` code points.
///
/// `Inner` is declared as a generic struct in order to take advantage of the
/// partial dynamically sized type (DST) support. For more information see:
/// <https://doc.rust-lang.org/nomicon/exotic-sizes.html#dynamically-sized-types-dsts>
#[repr(C)]
struct Inner<T: ?Sized> {
    len: usize,
    buf: T,
}

/// A borrowed AssemblyScript string.
///
/// Dynamically sized types are not safe over FFI boundries, so it is important
/// to only use `AscStr` (and not `AscString`) as exported and imported function
/// parameter types.
#[repr(transparent)]
pub struct AscStr {
    inner: Inner<[u16; 0]>,
}

impl AscStr {
    /// Converts the AssemblyScript string into a Rust `String`.
    #[allow(unused)] // TODO(nlordell): Remove once it is used.
    pub fn to_string(&self) -> Result<String, FromUtf16Error> {
        String::from_utf16(&self.as_code_points())
    }

    /// Converts the AssemblyScript string into a Rust `String`, replacing
    /// invalid data with the replacement character (`U+FFFD`).
    pub fn to_string_lossy(&self) -> String {
        String::from_utf16_lossy(&self.as_code_points())
    }

    /// Returns a slice of the utf-16 code points for this string.
    fn as_code_points(&self) -> &[u16] {
        unsafe { slice::from_raw_parts(&self.inner.buf as *const _, self.inner.len) }
    }
}

impl Debug for AscStr {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        Debug::fmt(&self.to_string_lossy(), f)
    }
}

/// An AssemblyScript string.
#[repr(transparent)]
pub struct AscString {
    inner: Inner<[u16]>,
}

impl AscString {
    /// Creates a new AssemblyScript string from a Rust string slice.
    pub fn new(s: impl AsRef<str>) -> Box<Self> {
        let s = s.as_ref();
        let len = s.encode_utf16().count();
        let mut string = unsafe {
            alloc_string(len)
                .expect("attempted to allocate a string that is larger than the address space.")
        };
        string.inner.len = len;
        for (i, c) in s.encode_utf16().enumerate() {
            string.inner.buf[i] = c;
        }

        string
    }

    /// Returns a reference to a borrowed AssemblyScript string.
    pub fn as_asc_str(&self) -> &AscStr {
        unsafe { &*(&self.inner.len as *const usize).cast::<AscStr>() }
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

/// Returns the memory layout for an AssemblyScript string.
fn string_layout(len: usize) -> Result<Layout, LayoutErr> {
    let (layout, _) = Layout::new::<usize>().extend(Layout::array::<u16>(len)?)?;
    // NOTE: Pad to alignment for C ABI compatibility. See
    // <https://doc.rust-lang.org/std/alloc/struct.Layout.html#method.extend>
    Ok(layout.pad_to_align())
}

/// A Rust dynamically sized type fat pointer.
struct DstRef {
    #[allow(dead_code)]
    ptr: *const u8,
    #[allow(dead_code)]
    len: usize,
}

/// Allocates an empty uninitialized AssemblyScript string with the
/// specified length.
unsafe fn alloc_string(len: usize) -> Result<Box<AscString>, LayoutErr> {
    // NOTE: Rust only has partial DST support, so we need to use some unsafe
    // magic to create a fat `Box` for a DST since there is currently no stable
    // safe way to create one otherwise.
    let string = mem::transmute(DstRef {
        ptr: alloc::alloc(string_layout(len)?),
        len,
    });

    Ok(string)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn can_round_trip_str() {
        let message = "Hello 🦀";
        assert_eq!(AscString::new(message).to_string().unwrap(), message);
    }

    #[test]
    fn string_layout_matches_type() {
        let string = AscString::new("1");
        let layout = Layout::for_value(&*string);
        assert_eq!(layout, string_layout(1).unwrap());
    }

    #[test]
    fn string_layout_matches_dst_layout() {
        assert_eq!(
            Layout::for_value(&*{
                let inner: Box<Inner<[u16]>> = Box::new(Inner {
                    len: 0,
                    buf: [0; 5],
                });
                inner
            }),
            string_layout(5).unwrap()
        );
        assert_eq!(
            Layout::for_value(&*{
                let inner: Box<Inner<[u16]>> = Box::new(Inner {
                    len: 0,
                    buf: [0; 8],
                });
                inner
            }),
            string_layout(8).unwrap()
        );
    }

    #[test]
    fn string_has_length_set() {
        let string = AscString::new("1");
        assert_eq!(string.inner.len, string.inner.buf.len());
        assert_eq!(string.inner.len, 1);
    }

    #[test]
    fn dst_ref_layout() {
        let inner: Box<Inner<[u16]>> = Box::new(Inner {
            len: 0,
            buf: [0; 5],
        });

        let inner_ptr = &inner.len as *const usize;
        let inner_ref: DstRef = unsafe { mem::transmute(inner) };

        assert_eq!(inner_ref.ptr, inner_ptr.cast::<u8>());
        assert_eq!(inner_ref.len, 5);

        mem::drop(unsafe { mem::transmute::<_, Box<Inner<[u16]>>>(inner_ref) });
    }

    #[test]
    #[should_panic]
    fn string_access_out_of_bounds() {
        let string = AscString::new("1");
        let _ = string.inner.buf[1];
    }
}
