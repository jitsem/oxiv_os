// Heavily Inspired by Stephen Marz's blog post: https://osblog.stephenmarz.com/ch3.html
use crate::arch::PAGE_ORDER;
use crate::arch::PAGE_SIZE;
use crate::{print, println, spinlock::SpinLock};

pub static mut PAGE_ALLOCATOR: SpinLock<PageAllocator> = SpinLock::new(PageAllocator::new());

/// Aligns a value to the next multiple of the order.
/// So, this is a little trick to make sure any address is aligned to a page boundary.
pub const fn align_val(val: usize, order: usize) -> usize {
    let order = (1 << order) - 1;
    (val + order) & !order
}

pub struct PageAllocator {
    heap_start: usize,
    alloc_start: usize,
    total_num_pages: usize,
}

#[repr(u8)]
enum PageState {
    Free = 0,
    Taken = 1 << 0,
    Last = 1 << 1,
}

#[repr(C)]
struct PageDescriptor {
    flags: u8,
}

impl PageState {
    fn to_u8(self) -> u8 {
        self as u8
    }
}

impl PageDescriptor {
    fn new() -> Self {
        PageDescriptor {
            flags: PageState::Free.to_u8(),
        }
    }

    fn clear(&mut self) {
        self.flags = PageState::Free.to_u8();
    }

    fn add_flag(&mut self, flag: PageState) {
        self.flags |= flag.to_u8();
    }

    fn is_free(&self) -> bool {
        !self.is_taken()
    }

    fn is_taken(&self) -> bool {
        self.flags & PageState::Taken.to_u8() != 0
    }

    fn is_last(&self) -> bool {
        self.flags & PageState::Last.to_u8() != 0
    }
}

impl PageAllocator {
    pub const fn new() -> Self {
        PageAllocator {
            heap_start: 0,
            total_num_pages: 0,
            alloc_start: 0,
        }
    }

    pub fn init(&mut self, heap_start: usize, heap_end: usize) {
        let size = heap_end - heap_start;
        let total_num_pages = size / PAGE_SIZE;
        //Clear all pages

        let alloc_start = align_val(
            heap_start + (total_num_pages * size_of::<PageDescriptor>()),
            PAGE_ORDER,
        );
        self.heap_start = heap_start;
        self.total_num_pages = total_num_pages;
        self.alloc_start = alloc_start;

        for i in 0..self.total_num_pages {
            let page = self.heap_start + i * PAGE_SIZE;
            let pd = page as *mut PageDescriptor;
            unsafe {
                pd.write(PageDescriptor::new());
            }
        }
    }

    pub fn alloc(&mut self, nr_of_pages: usize) -> *mut u8 {
        let page = self.heap_start;
        let mut pages_found = 0;
        let mut pd = page as *mut PageDescriptor;
        let mut found_index = (false, 0);
        for i in 0..self.total_num_pages - nr_of_pages {
            unsafe {
                if (*pd).is_free() {
                    pages_found += 1;
                    if pages_found == nr_of_pages {
                        found_index = (true, i - nr_of_pages + 1);
                        break;
                    }
                } else {
                    pages_found = 0;
                }
                pd = pd.add(1);
            }
        }
        if !found_index.0 {
            return core::ptr::null_mut();
        }
        unsafe {
            (*pd).add_flag(PageState::Last);
            pd = pd.sub(nr_of_pages);
            for i in 0..=nr_of_pages {
                (*pd.add(i)).add_flag(PageState::Taken);
            }
        }

        //The function needs to return the address of the first page, not the descriptor
        (self.alloc_start + found_index.1 * PAGE_SIZE) as *mut u8
    }

    pub fn dealloc(&mut self, page: *mut u8) {
        //Check if the page is within the bounds of the allocator
        let addr = self.heap_start + (page as usize - self.alloc_start) / PAGE_SIZE;

        if addr < self.heap_start || addr >= self.alloc_start {
            panic!("Page {:#x} is not within the bounds of the allocator. Calculated descriptor was {:#x}", page as usize, addr);
        }

        //Clear all taken descriptors that are not marked as last
        let mut pd = addr as *mut PageDescriptor;
        unsafe {
            while !(*pd).is_last() && (*pd).is_taken() {
                (*pd).clear();
                pd = pd.add(1);
            }
            //Guard against deallocating a page that is not marked as last since that indicates a double frees
            assert!((*pd).is_last(), "Page is not marked as last");
            //Clear the last page
            (*pd).clear();
        }
    }

    pub fn zero_alloc(&mut self, pages: usize) -> *mut u8 {
        let page = self.alloc(pages);
        if page.is_null() {
            return core::ptr::null_mut();
        }
        unsafe {
            core::ptr::write_bytes(page, 0, pages * PAGE_SIZE);
        }
        page
    }

    pub fn print_page_allocations(&self) {
        println!(
            "PageAllocator: heap_start: {:#x}, alloc_start: {:#x}, total_num_pages: {}",
            self.heap_start, self.alloc_start, self.total_num_pages
        );
        let mut in_block = false;
        for i in 0..self.total_num_pages {
            let pd = (self.heap_start + i * size_of::<PageDescriptor>()) as *const PageDescriptor;
            unsafe {
                if (*pd).is_taken() {
                    if !in_block {
                        print!("Block start: ");
                        in_block = true;
                    }
                    print!("{:#x} ", self.heap_start + i * PAGE_SIZE);
                    if (*pd).is_last() {
                        println!("Done");
                        in_block = false;
                    }
                }
            }
        }
    }
}

//To satisfy clippy
impl Default for PageAllocator {
    fn default() -> Self {
        Self::new()
    }
}
