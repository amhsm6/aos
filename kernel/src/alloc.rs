extern crate alloc;

use core::cell::UnsafeCell;
use alloc::alloc::{GlobalAlloc, Layout};

struct Allocator {
    top: UnsafeCell<*mut u8>
}

unsafe impl Sync for Allocator {}

unsafe impl GlobalAlloc for Allocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let offset = (*self.top.get()).align_offset(layout.align());
        let ptr = (*self.top.get()).add(offset);

        let top = ptr.add(layout.size());
        *self.top.get() = top;

        ptr
    }

    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {}
}

#[global_allocator]
static ALLOCATOR: Allocator = Allocator { top: UnsafeCell::new(0x7fffff as *mut u8) };
