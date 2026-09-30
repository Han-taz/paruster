//! Per-isolate limit for V8-owned ArrayBuffer backing stores.
//!
//! V8 also allocates heap and native memory outside this allocator. This cap
//! is one layer of the private probe, not a process RSS limit.

use std::alloc::{Layout, alloc, alloc_zeroed, dealloc};
use std::ffi::c_void;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

const ALIGNMENT: usize = 16;

pub(super) struct AllocationCap {
    limit: usize,
    used: AtomicUsize,
    rejected: AtomicBool,
}

impl AllocationCap {
    pub(super) fn new(limit: usize) -> Arc<Self> {
        Arc::new(Self {
            limit,
            used: AtomicUsize::new(0),
            rejected: AtomicBool::new(false),
        })
    }

    pub(super) fn used(&self) -> usize {
        self.used.load(Ordering::Acquire)
    }

    pub(super) fn rejected(&self) -> bool {
        self.rejected.load(Ordering::Acquire)
    }

    pub(super) fn can_allocate(&self, bytes: usize) -> bool {
        self.used()
            .checked_add(bytes.max(1))
            .is_some_and(|total| total <= self.limit)
    }

    fn reserve(&self, bytes: usize) -> bool {
        let charge = bytes.max(1);
        let result = self
            .used
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |used| {
                used.checked_add(charge)
                    .filter(|total| *total <= self.limit)
            });
        if result.is_err() {
            self.rejected.store(true, Ordering::Release);
            return false;
        }
        true
    }

    fn release(&self, bytes: usize) {
        let previous = self.used.fetch_sub(bytes.max(1), Ordering::AcqRel);
        debug_assert!(previous >= bytes.max(1));
    }
}

unsafe extern "C" fn allocate(handle: &AllocationCap, len: usize) -> *mut c_void {
    allocate_impl(handle, len, true)
}

unsafe extern "C" fn allocate_uninitialized(handle: &AllocationCap, len: usize) -> *mut c_void {
    allocate_impl(handle, len, false)
}

fn allocate_impl(handle: &AllocationCap, len: usize, zeroed: bool) -> *mut c_void {
    if !handle.reserve(len) {
        return std::ptr::null_mut();
    }
    let Ok(layout) = Layout::from_size_align(len.max(1), ALIGNMENT) else {
        handle.release(len);
        handle.rejected.store(true, Ordering::Release);
        return std::ptr::null_mut();
    };
    // SAFETY: `layout` has nonzero size and valid alignment. The matching
    // `free` callback deallocates with the same length and alignment.
    let ptr = unsafe {
        if zeroed {
            alloc_zeroed(layout)
        } else {
            alloc(layout)
        }
    };
    if ptr.is_null() {
        handle.release(len);
        handle.rejected.store(true, Ordering::Release);
    }
    ptr.cast()
}

unsafe extern "C" fn free(handle: &AllocationCap, data: *mut c_void, len: usize) {
    if data.is_null() {
        return;
    }
    let layout = Layout::from_size_align(len.max(1), ALIGNMENT)
        .expect("V8 must free with the length supplied at allocation");
    // SAFETY: V8 calls this callback once for the pointer returned by our
    // allocator and passes its original length.
    unsafe { dealloc(data.cast(), layout) };
    handle.release(len);
}

unsafe extern "C" fn drop_handle(handle: *const AllocationCap) {
    // SAFETY: exactly one Arc strong reference was passed to V8 by
    // `Arc::into_raw` in `new_v8_allocator`.
    unsafe { drop(Arc::from_raw(handle)) };
}

static VTABLE: v8::RustAllocatorVtable<AllocationCap> = v8::RustAllocatorVtable {
    allocate,
    allocate_uninitialized,
    free,
    drop: drop_handle,
};

pub(super) fn new_v8_allocator(cap: &Arc<AllocationCap>) -> v8::UniqueRef<v8::Allocator> {
    // SAFETY: the Arc keeps the handle alive until V8 invokes VTABLE.drop;
    // all callback functions match the handle type and are thread-safe.
    unsafe { v8::new_rust_allocator(Arc::into_raw(Arc::clone(cap)), &VTABLE) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_limit_rejects_next_byte_and_free_releases_charge() {
        let cap = AllocationCap::new(17);
        let p = allocate_impl(&cap, 16, true);
        assert!(!p.is_null());
        assert_eq!(cap.used(), 16);
        assert!(cap.can_allocate(1));
        let q = allocate_impl(&cap, 1, false);
        assert!(!q.is_null());
        assert_eq!(cap.used(), 17);
        assert!(!cap.can_allocate(1));
        let denied = allocate_impl(&cap, 1, false);
        assert!(denied.is_null());
        assert!(cap.rejected());
        assert_eq!(cap.used(), 17);
        // SAFETY: both pointers came from `allocate_impl` and are freed once
        // with their original lengths.
        unsafe {
            free(&cap, q, 1);
            free(&cap, p, 16);
        }
        assert_eq!(cap.used(), 0);
    }
}
