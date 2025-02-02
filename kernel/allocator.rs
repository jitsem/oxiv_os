use crate::arch::PAGE_ORDER;
use crate::arch::PAGE_SIZE;
use core::alloc::GlobalAlloc;

use crate::page::{self};

extern crate alloc;

pub struct KernelAllocator;

unsafe impl GlobalAlloc for KernelAllocator {
    unsafe fn alloc(&self, layout: core::alloc::Layout) -> *mut u8 {
        let nr_of_pages = page::align_val(layout.size(), PAGE_ORDER) / PAGE_SIZE;
        page::PAGE_ALLOCATOR.lock().alloc(nr_of_pages)
    }

    unsafe fn dealloc(&self, ptr: *mut u8, _layout: core::alloc::Layout) {
        page::PAGE_ALLOCATOR.lock().dealloc(ptr);
    }
}
