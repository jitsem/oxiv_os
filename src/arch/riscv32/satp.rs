use crate::arch::PAGE_ORDER;
use core::arch::global_asm;

pub struct Satp {
    value: usize,
}
impl Satp {
    pub const fn new(ppn: usize) -> Self {
        let value = 1usize << 31 | (ppn as usize >> PAGE_ORDER);
        Satp { value }
    }

    pub fn get(&self) -> usize {
        self.value
    }

    pub fn switch(&self) {
        unsafe {
            __set_satp(self.value);
        }
    }
}

extern "C" {
    fn __set_satp(satp: usize);
}
global_asm!("__set_satp:", "csrw satp, a0", "sfence.vma", "ret");
