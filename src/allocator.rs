use core::alloc::GlobalAlloc;

use crate::{
    page::{self, PageAllocator, PAGE_SIZE},
    spinlock::SpinLock,
};

extern crate alloc;

pub struct KernelAllocator {
    alloc: Option<&'static SpinLock<PageAllocator>>,
}

impl KernelAllocator {
    pub const fn new() -> Self {
        KernelAllocator { alloc: None }
    }

    pub fn init(&mut self, alloc: &'static SpinLock<PageAllocator>) {
        self.alloc = Some(alloc);
    }
}

unsafe impl GlobalAlloc for KernelAllocator {
    unsafe fn alloc(&self, layout: core::alloc::Layout) -> *mut u8 {
        let nr_of_pages = page::align_val(layout.size(), page::PAGE_ORDER) / PAGE_SIZE;
        self.alloc.unwrap().lock().alloc(nr_of_pages)
    }

    unsafe fn dealloc(&self, ptr: *mut u8, _layout: core::alloc::Layout) {
        self.alloc.unwrap().lock().dealloc(ptr);
    }
}
