//! AssemblyScript dynamically sized buffer implementation.

use std::{
    alloc::{self, Layout, LayoutErr},
    borrow::{Borrow, ToOwned},
    fmt::{self, Debug, Formatter},
    marker::PhantomData,
    mem,
    ops::Deref,
    ptr, slice,
};

/// Internal representation of an AssemblyScript buffer.
///
/// `Inner` is declared as a generic struct in order to take advantage of the
/// partial dynamically sized type (DST) support. For more information see:
/// <https://doc.rust-lang.org/nomicon/exotic-sizes.html#dynamically-sized-types-dsts>
#[repr(C)]
struct Inner<T: ?Sized> {
    len: usize,
    buf: T,
}

/// A borrowed AssemblyScript dynamically sized buffer with elements of type `T`
/// and aligned to `Alignment`.
#[repr(transparent)]
pub struct AscBuf<T, Alignment = T> {
    _type: PhantomData<T>,
    inner: Inner<[Alignment; 0]>,
}

impl<T, A> AscBuf<T, A> {
    /// Returns the number of elements in the buffer.
    pub fn len(&self) -> usize {
        self.inner.len
    }

    /// Returns the buffer as a Rust slice.
    pub fn as_slice(&self) -> &[T] {
        let Inner { len, buf } = &self.inner;

        // SAFETY: `AscBuf` can only be constructed from an `AscBuffer` which
        // correctly allocates the storage starting at `buf` to have `len`
        // elements. Additionally we *assume* that all `AscBuf` references from
        // host calls are valid in the same way.
        unsafe { slice::from_raw_parts((buf as *const A).cast(), *len) }
    }
}

impl<T, A> AsRef<[T]> for AscBuf<T, A> {
    fn as_ref(&self) -> &[T] {
        self.as_slice()
    }
}

impl<T, A> Debug for AscBuf<T, A>
where
    T: Debug,
{
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        Debug::fmt(&self.as_slice(), f)
    }
}

impl<T, A> ToOwned for AscBuf<T, A> {
    type Owned = Box<AscBuffer<T, A>>;

    fn to_owned(&self) -> Self::Owned {
        AscBuffer::new(self)
    }
}

/// An owned AssemblyScript dynamically sized buffer with elements of type `T`
/// and aligned to `Alignment`.
pub struct AscBuffer<T, Alignment = T> {
    _type: PhantomData<T>,
    inner: Inner<[Alignment]>,
}

impl<T, A> AscBuffer<T, A> {
    /// Creates a new AssemblyScript buffer from the specified slice.
    pub fn new(slice: impl AsRef<[T]>) -> Box<Self> {
        let slice = slice.as_ref();

        // SAFETY: The allocated buffer is guaranteed to be completely
        // initialized.
        unsafe {
            let mut buffer = alloc_buffer::<T, A>(slice.len());
            buffer.inner.len = slice.len();

            // NOTE: Use `ptr::copy` here since the allocated buffer contains
            // unintialized memory. It is considered undefined behaviour to
            // create a reference to uninitialized memory in Rust.
            ptr::copy(
                slice.as_ptr(),
                buffer.inner.buf.as_mut_ptr().cast(),
                slice.len(),
            );

            buffer
        }
    }

    /// Returns a reference to a borrowed AssemblyScript buffer.
    pub fn as_buf(&self) -> &AscBuf<T, A> {
        unsafe { &*(&self.inner.len as *const usize).cast::<AscBuf<T, A>>() }
    }

    /// Returns an FFI-safe pointer to an AssemblyScript buffer.
    pub fn as_buf_ptr(&self) -> *const AscBuf<T, A> {
        self.as_buf() as *const _
    }
}

impl<T, A> Borrow<AscBuf<T, A>> for Box<AscBuffer<T, A>> {
    fn borrow(&self) -> &AscBuf<T, A> {
        self.as_buf()
    }
}

impl<T, A> Deref for AscBuffer<T, A> {
    type Target = AscBuf<T, A>;

    fn deref(&self) -> &Self::Target {
        self.as_buf()
    }
}

impl<T, A> Debug for AscBuffer<T, A>
where
    T: Debug,
{
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        Debug::fmt(self.as_buf(), f)
    }
}

/// Returns the memory layout for an AssemblyScript buffer with the specified
/// dynamic length.
fn buffer_layout<T, A>(len: usize) -> Result<Layout, LayoutErr> {
    let (layout, _) = Layout::new::<AscBuf<T, A>>().extend(Layout::array::<T>(len)?)?;
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

/// Allocates an empty uninitialized AssemblyScript buffer with the specified
/// dynamic length.
///
/// # Safety
///
/// This method returns an *uninitialized* AssemblyScript buffer. It is
/// undefined behaviour to use if without proper initialization.
unsafe fn alloc_buffer<T, A>(len: usize) -> Box<AscBuffer<T, A>> {
    let layout = buffer_layout::<T, A>(len)
        .expect("attempted to allocate a buffer that is larger than the address space.");

    // NOTE: Rust only has partial DST support, so we need to use some unsafe
    // magic to create a fat `Box` for a DST since there is currently no stable
    // safe way to create one otherwise.
    mem::transmute(DstRef {
        ptr: alloc::alloc(layout),
        // NOTE: Guaranteed not to overflow, or else creating the layout would
        // have errored.
        len: ceil_div(len * mem::size_of::<T>(), mem::size_of::<A>()),
    })
}

/// Ceiling integer usize division.
fn ceil_div(n: usize, d: usize) -> usize {
    (n + d - 1) / d
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn buffer_layout_matches_type() {
        let buffer = AscBuffer::<u8, u64>::new([1u8]);
        let layout = Layout::for_value(&*buffer);
        assert_eq!(layout, buffer_layout::<u8, u64>(1).unwrap());
    }

    #[test]
    fn buffer_layout_matches_dst_layout() {
        assert_eq!(
            Layout::for_value(&*{
                let inner: Box<Inner<[u16]>> = Box::new(Inner {
                    len: 0,
                    buf: [0; 5],
                });
                inner
            }),
            buffer_layout::<u16, usize>(5).unwrap()
        );
        assert_eq!(
            Layout::for_value(&*{
                let inner: Box<Inner<[u16]>> = Box::new(Inner {
                    len: 0,
                    buf: [0; 8],
                });
                inner
            }),
            buffer_layout::<u16, usize>(8).unwrap()
        );
    }

    #[test]
    fn buffer_has_length_set_to_alignment() {
        let buffer = AscBuffer::<u16, u64>::new([42, 1337]);
        assert_eq!(buffer.inner.buf.len(), 1);
        assert_eq!(buffer.inner.len, 2);
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
    fn buffer_access_out_of_bounds() {
        let buffer = AscBuffer::<u32, usize>::new([0]);
        let _ = buffer.inner.buf[1];
    }

    #[test]
    fn owned_and_borrowed_layout() {
        let buf = AscBuf {
            _type: PhantomData::<u8>,
            inner: Inner::<[u64; 0]> { len: 0, buf: [] },
        };

        let empty_buffer = AscBuffer::<u8, u64>::new([]);
        assert_eq!(Layout::for_value(&*empty_buffer), Layout::for_value(&buf));

        let buffer = AscBuffer::<u8, u64>::new([0]);
        assert_eq!(
            ptr_offset(&buffer.inner.len, &buffer.inner.buf[0]),
            ptr_offset(&buf.inner.len, &buf.inner.buf),
        );
    }

    fn ptr_offset<T, U>(x: &T, y: &U) -> isize {
        ((x as *const T) as isize) - ((y as *const U) as isize)
    }
}
