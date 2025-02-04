extern crate alloc;

use core::ptr;
use core::cell::Cell;
use alloc::alloc::{GlobalAlloc, Layout};

extern "C" {
    #[link_name = "_heap_begin"]
    static HEAP_BEGIN: u64;

    #[link_name = "_heap_end"]
    static HEAP_END: u64;
}

struct Allocator {
    top: Cell<*mut u8>
}

unsafe impl Sync for Allocator {}

unsafe impl GlobalAlloc for Allocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let top = self.top.get();

        let offset = top.align_offset(layout.align());
        let ptr = top.add(offset);

        let newtop = ptr.add(layout.size());
        self.top.replace(newtop);

        ptr
    }

    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {}
}

#[global_allocator]
static ALLOCATOR: Allocator = Allocator { top: Cell::new(ptr::addr_of!(HEAP_BEGIN) as *mut u8) };
