use core::{alloc::GlobalAlloc, cell::UnsafeCell};

use crate::spinlock::SpinLock;

extern crate alloc;

pub type LockBumpAllocator = SpinLock<BumpAllocator>;

unsafe impl GlobalAlloc for LockBumpAllocator {
    unsafe fn alloc(&self, layout: core::alloc::Layout) -> *mut u8 {
        self.lock().alloc(layout)
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: core::alloc::Layout) {
        self.lock().dealloc(ptr, layout);
    }
}
pub struct BumpAllocator {
    state: Option<BumpAllocState>,
}

struct BumpAllocState {
    heap_end: usize,
    next: UnsafeCell<usize>,
}

impl BumpAllocator {
    pub const fn empty() -> Self {
        BumpAllocator { state: None }
    }
    pub fn init(&mut self, heap_start: usize, heap_end: usize) {
        self.state = Some(BumpAllocState {
            heap_end,
            next: UnsafeCell::new(heap_start),
        });
    }
}

unsafe impl GlobalAlloc for BumpAllocator {
    unsafe fn alloc(&self, layout: core::alloc::Layout) -> *mut u8 {
        if let Some(state) = &self.state {
            let alloc_start = *state.next.get();
            let alloc_end = *state.next.get() + layout.size();
            if alloc_end > state.heap_end {
                panic!("Out of memory. Couldn't alloc for size: {}!", layout.size())
            } else {
                *(state.next.get()) = alloc_end;
                alloc_start as *mut u8
            }
        } else {
            panic!("Allocator was not inited!!!")
        }
    }

    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: core::alloc::Layout) {
        //We do not de-alloc in a bump allocator
    }
}
