//! This module contains read only types to interop with C++ std collections.

use std::{ffi, marker::PhantomData};

#[repr(C)]
/// Struct layout for the GCC C++ std::string.
/// NOTE: THIS IS ONLY FOR GCC, EVERY COMPILER IS DIFFERENT.
pub struct CppString {
    data: *mut ffi::c_char,
    size: usize,
    rest: CapacityOrBuf,
}

#[allow(dead_code)]
union CapacityOrBuf {
    pub cap: usize,
    pub buf: [ffi::c_char; 16],
}

impl CppString {
    /// Gets a [`ffi::CStr`] from `self`'s data.
    /// # Safety
    /// The data within this string should have been created by C++ code, which will enforce null terminatation.
    #[inline]
    pub unsafe fn c_str(&self) -> &ffi::CStr {
        ffi::CStr::from_ptr(self.data)
    }

    /// Get the length of `self`.
    #[inline]
    pub fn len(&self) -> usize {
        self.size
    }

    /// Return true if `self` is empty.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.size == 0
    }
}

/// Struct layout for the GCC C++ std::vector. NOTE: Might work on more compilers
/// since the vector layout is more standardized.
#[repr(C)]
pub struct CppVector<T> {
    begin: *mut T,
    end: *const T,
    end_cap: *const T,
    _t: PhantomData<T>,
}

impl<T> CppVector<T> {
    /// Creates a new empty C++ vector.
    pub fn new() -> Self {
        Self {
            begin: std::ptr::null_mut(),
            end: std::ptr::null(),
            end_cap: std::ptr::null(),
            _t: PhantomData,
        }
    }
}

impl<T> Default for CppVector<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a, T: 'a> IntoIterator for &'a CppVector<T> {
    type Item = *const T;
    type IntoIter = CppVectorIter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        CppVectorIter {
            inner: self,
            curr: self.begin,
        }
    }
}

/// Represents an iterator over a [`CppVector`].
pub struct CppVectorIter<'a, T> {
    inner: &'a CppVector<T>,
    curr: *const T,
}

impl<T> Iterator for CppVectorIter<'_, T> {
    type Item = *const T;

    fn next(&mut self) -> Option<Self::Item> {
        if self.curr >= self.inner.end {
            return None;
        }

        let out = self.curr;
        self.curr = unsafe { self.curr.offset(1) };
        Some(out)
    }
}
