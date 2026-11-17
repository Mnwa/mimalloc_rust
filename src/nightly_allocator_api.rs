use crate::MiMalloc;
use core::alloc::{AllocError, Allocator, Layout};
use core::ffi::c_void;
use core::ptr::NonNull;
use ffi::*;

impl MiMalloc {
    /// Interprets a raw pointer returned from a memory allocation call.
    /// Tags the pointer with the associated allocation size,
    /// or returns [`AllocError`] if the pointer was null.
    ///
    /// # Safety
    ///
    /// The `raw_ptr` must have been returned by mimalloc.
    /// It should either refer to a live allocation or be null.
    #[inline]
    unsafe fn tag_allocation(raw_ptr: *mut c_void) -> Result<NonNull<[u8]>, AllocError> {
        if let Some(ptr) = NonNull::new(raw_ptr as *mut _) {
            // Safety: `raw_ptr` was previously allocated with mimalloc
            let len = unsafe { mi_usable_size(raw_ptr) };
            Ok(NonNull::from_raw_parts(ptr, len))
        } else {
            Err(AllocError)
        }
    }
}

unsafe impl Allocator for MiMalloc {
    #[inline]
    fn allocate(&self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        match layout.size() {
            0 => Ok(NonNull::slice_from_raw_parts(layout.dangling_ptr(), 0)),
            // Safety: the pointer passed to `tag_allocation` is either null or comes from mimalloc
            _ => unsafe { Self::tag_allocation(mi_malloc_aligned(layout.size(), layout.align())) },
        }
    }

    #[inline]
    unsafe fn deallocate(&self, ptr: NonNull<u8>, layout: Layout) {
        match layout.size() {
            // Do nothing
            0 => {}
            // Safety: by the function preconditions, `ptr` came from this allocator
            _ => unsafe { mi_free(ptr.as_ptr() as *mut c_void) },
        }
    }

    #[inline]
    fn allocate_zeroed(&self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        match layout.size() {
            0 => Ok(NonNull::slice_from_raw_parts(layout.dangling_ptr(), 0)),
            // Safety: the pointer passed to `tag_allocation` is either null or comes from mimalloc
            _ => unsafe { Self::tag_allocation(mi_zalloc_aligned(layout.size(), layout.align())) },
        }
    }

    #[inline]
    unsafe fn grow(
        &self,
        ptr: NonNull<u8>,
        old_layout: Layout,
        new_layout: Layout,
    ) -> Result<NonNull<[u8]>, AllocError> {
        match (old_layout.size(), new_layout.size()) {
            // Do nothing
            (0, 0) => Ok(NonNull::slice_from_raw_parts(new_layout.dangling_ptr(), 0)),
            // Safety: by the function preconditions, `ptr` came from this allocator
            (0, _) => self.allocate(new_layout),
            // Safety: by the function preconditions, `ptr` came from this allocator
            (_, _) => unsafe {
                Self::tag_allocation(mi_realloc_aligned(
                    ptr.as_ptr() as *mut c_void,
                    new_layout.size(),
                    new_layout.align(),
                ))
            },
        }
    }

    #[inline]
    unsafe fn grow_zeroed(
        &self,
        ptr: NonNull<u8>,
        old_layout: Layout,
        new_layout: Layout,
    ) -> Result<NonNull<[u8]>, AllocError> {
        match old_layout.size() {
            // `ptr` is dangling, there is nothing to reallocate
            0 => self.allocate_zeroed(new_layout),
            // Safety: by the function preconditions, `ptr` came from this allocator.
            // `mi_rezalloc_aligned` preserves the old usable bytes and zeroes
            // everything up to the usable size of the new block.
            _ => unsafe {
                Self::tag_allocation(mi_rezalloc_aligned(
                    ptr.as_ptr() as *mut c_void,
                    new_layout.size(),
                    new_layout.align(),
                ))
            },
        }
    }

    #[inline]
    unsafe fn shrink(
        &self,
        ptr: NonNull<u8>,
        old_layout: Layout,
        new_layout: Layout,
    ) -> Result<NonNull<[u8]>, AllocError> {
        match (old_layout.size(), new_layout.size()) {
            // Do nothing
            (0, 0) => Ok(NonNull::slice_from_raw_parts(new_layout.dangling_ptr(), 0)),
            // Safety: by the function preconditions, `ptr` came from this allocator
            (_, 0) => unsafe {
                self.deallocate(ptr, old_layout);
                Ok(NonNull::slice_from_raw_parts(new_layout.dangling_ptr(), 0))
            },
            // Safety: by the function preconditions, `ptr` came from this allocator
            (_, _) => unsafe {
                Self::tag_allocation(mi_realloc_aligned(
                    ptr.as_ptr() as *mut _,
                    new_layout.size(),
                    new_layout.align(),
                ))
            },
        }
    }
}

#[cfg(test)]
mod tests {
    extern crate alloc;

    use super::*;
    use alloc::boxed::Box;
    use alloc::vec::Vec;
    use core::mem::ManuallyDrop;

    #[test]
    fn it_frees_allocated_memory() {
        let layout = Layout::from_size_align(8, 8).unwrap();
        let ptr = MiMalloc.allocate(layout).unwrap();
        unsafe { MiMalloc.deallocate(ptr.cast(), layout) };
    }

    #[test]
    fn it_frees_allocated_big_memory() {
        let layout = Layout::from_size_align(1 << 20, 32).unwrap();
        let ptr = MiMalloc.allocate(layout).unwrap();
        unsafe { MiMalloc.deallocate(ptr.cast(), layout) };
    }

    #[test]
    fn it_frees_zero_allocated_memory() {
        let layout = Layout::from_size_align(8, 8).unwrap();
        let ptr = MiMalloc.allocate_zeroed(layout).unwrap();
        unsafe { MiMalloc.deallocate(ptr.cast(), layout) };
    }

    #[test]
    fn it_frees_zero_allocated_big_memory() {
        let layout = Layout::from_size_align(1 << 20, 32).unwrap();
        let ptr = MiMalloc.allocate_zeroed(layout).unwrap();
        unsafe { MiMalloc.deallocate(ptr.cast(), layout) };
    }

    #[test]
    fn it_frees_grown_memory() {
        let layout = Layout::from_size_align(8, 8).unwrap();
        let new_layout = Layout::from_size_align(16, 8).unwrap();
        let ptr = MiMalloc.allocate(layout).unwrap();
        let ptr = unsafe { MiMalloc.grow(ptr.cast(), layout, new_layout).unwrap() };
        unsafe { MiMalloc.deallocate(ptr.cast(), new_layout) };
    }

    #[test]
    fn it_frees_grown_big_memory() {
        let layout = Layout::from_size_align(1 << 20, 32).unwrap();
        let new_layout = Layout::from_size_align(2 << 20, 32).unwrap();
        let ptr = MiMalloc.allocate(layout).unwrap();
        let ptr = unsafe { MiMalloc.grow(ptr.cast(), layout, new_layout).unwrap() };
        unsafe { MiMalloc.deallocate(ptr.cast(), new_layout) };
    }

    /// Allocates a zeroed vector, fills it with a marker, grows it with
    /// `Allocator::grow_zeroed` and checks the contents of the resized vector.
    fn check_grow_zeroed(old_len: usize, new_len: usize) {
        const MARKER: u8 = 0xAB;

        // Safety: all-zero bytes are a valid `u8`
        let mut vec: Vec<u8, MiMalloc> =
            unsafe { Box::new_zeroed_slice_in(old_len, MiMalloc).assume_init() }.into_vec();
        assert_eq!(vec.len(), old_len);
        assert!(vec.iter().all(|&b| b == 0));
        vec.fill(MARKER);

        let mut vec = ManuallyDrop::new(vec);
        let old_layout = Layout::array::<u8>(vec.capacity()).unwrap();
        let new_layout = Layout::array::<u8>(new_len).unwrap();
        let ptr = NonNull::new(vec.as_mut_ptr()).unwrap();

        // Safety: `ptr` was allocated by `MiMalloc` with `old_layout`
        let grown = unsafe { MiMalloc.grow_zeroed(ptr, old_layout, new_layout).unwrap() };
        assert!(grown.len() >= new_len);

        // Safety: `grown` holds `new_len` initialized bytes and was allocated with `new_layout`
        let vec = unsafe {
            Vec::from_raw_parts_in(grown.cast::<u8>().as_ptr(), new_len, new_len, MiMalloc)
        };
        assert!(vec[..old_len].iter().all(|&b| b == MARKER));
        assert!(vec[old_len..].iter().all(|&b| b == 0));
    }

    #[test]
    fn grow_zeroed_zeroes_new_memory() {
        check_grow_zeroed(8, 100);
    }

    #[test]
    fn grow_zeroed_zeroes_new_big_memory() {
        check_grow_zeroed(1 << 20, 2 << 20);
    }

    #[test]
    fn grow_zeroed_within_usable_size() {
        // 10 bytes land in a bigger size class, so growing to 12 fits in place
        check_grow_zeroed(10, 12);
    }

    #[test]
    fn grow_zeroed_from_empty() {
        check_grow_zeroed(0, 64);
    }

    #[test]
    fn empty_allocation_has_size_zero() {
        let layout = Layout::from_size_align(0, 1).unwrap();
        let ptr = MiMalloc.allocate(layout).unwrap();
        assert_eq!(ptr.len(), 0);
        unsafe { MiMalloc.deallocate(ptr.cast(), layout) };
    }

    #[test]
    fn shrink_to_empty_has_size_zero() {
        let layout = Layout::from_size_align(8, 8).unwrap();
        let empty_layout = Layout::from_size_align(0, 8).unwrap();
        let ptr = MiMalloc.allocate(layout).unwrap();
        let ptr = unsafe { MiMalloc.shrink(ptr.cast(), layout, empty_layout).unwrap() };
        assert_eq!(ptr.len(), 0);
        unsafe { MiMalloc.deallocate(ptr.cast(), empty_layout) };
    }
}
