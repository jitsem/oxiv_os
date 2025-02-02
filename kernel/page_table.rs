use crate::arch::PAGE_SIZE;
use crate::page;
use crate::println;

//See SV32 RISC-V Privileged ISA document
const ENTRIES_PER_TABLE: usize = 1024; // 2^10 entries per table
pub struct VirtualAddress(pub usize);
pub struct PhysicalAddress(u64);
impl VirtualAddress {
    fn as_usize(&self) -> usize {
        self.0
    }

    fn vpn1(&self) -> usize {
        // 0x3FF is 10 bits of 1s.
        // So what we are doing is right shifting by 22 bits and then masking the last 10 bits,
        // which gives us the first 10 bits of the address.
        (self.0 >> 22) & 0x3FF
    }

    fn vpn0(&self) -> usize {
        // 0x3FF is 10 bits of 1s.
        // So what we are doing is right shifting by 12 bits and then masking the last 10 bits,
        // which gives us the second 10 bits of the address.
        (self.0 >> 12) & 0x3FF
    }

    fn is_aligned(&self) -> bool {
        self.0 % PAGE_SIZE == 0
    }

    pub fn with_offset(&self, offset: usize) -> VirtualAddress {
        VirtualAddress(offset + self.0)
    }
}

impl PhysicalAddress {
    fn as_u64(&self) -> u64 {
        self.0
    }

    fn to_ppn(&self) -> u32 {
        let to_34 = self.0 & 0x3fffff000; // 34 bit
        (to_34 >> 2) as u32
    }

    fn is_aligned(&self) -> bool {
        self.0 % PAGE_SIZE as u64 == 0
    }

    pub fn with_offset(&self, offset: u64) -> PhysicalAddress {
        PhysicalAddress(offset + self.0)
    }
}

#[repr(usize)]
pub enum EntryFlags {
    None = 0,
    Valid = 1 << 0,
    Read = 1 << 1,
    Write = 1 << 2,
    Execute = 1 << 3,
    User = 1 << 4,
    Global = 1 << 5,
    Accessed = 1 << 6,
    Dirty = 1 << 7,
}

#[repr(C)]
#[repr(align(4096))]
pub struct PageTable {
    entries: [Entry; ENTRIES_PER_TABLE],
}

#[repr(C)]
#[derive(Copy, Clone)]
struct Entry(usize);

impl Entry {
    fn is_valid(&self) -> bool {
        self.0 & EntryFlags::Valid as usize != 0
    }
    fn get_phys_address(&self) -> PhysicalAddress {
        PhysicalAddress(((self.0 & 0xfffffc00) as u64) << 2)
    }

    fn is_leaf(&self) -> bool {
        //A entry is a leaf if it has Read Write or Execute set
        self.0
            & (EntryFlags::Read as usize
                | EntryFlags::Write as usize
                | EntryFlags::Execute as usize)
            != 0
    }

    fn is_branch(&self) -> bool {
        //Check if Read Write or Execute is not set
        !self.is_leaf()
    }
}
//TODO remove once I have some sort of mutex-init like thing?
#[allow(static_mut_refs)]
impl PageTable {
    pub const fn new() -> Self {
        PageTable {
            entries: [Entry(0); ENTRIES_PER_TABLE],
        }
    }

    pub fn map(
        &mut self,
        virt_address: VirtualAddress,
        phys_address: PhysicalAddress,
        flags: usize,
    ) {
        //Check address alignment
        assert!(
            virt_address.is_aligned(),
            "virt_address is not aligned: {:#x}",
            virt_address.as_usize()
        );
        assert!(
            phys_address.is_aligned(),
            "phys_address is not aligned: {:#x}",
            phys_address.as_u64()
        );

        let level1 = &mut self.entries[virt_address.vpn1()];
        let new_table = if !level1.is_valid() {
            // Allocate a new page table
            let new_table: *mut u8 = unsafe { page::PAGE_ALLOCATOR.lock().zero_alloc(1) };
            level1.0 = (new_table as usize >> 12) << 10 | EntryFlags::Valid as usize;
            new_table
        } else {
            // Get the existing page table from the level 1 entry
            (level1.get_phys_address().0) as *mut u8
        };
        unsafe {
            (*(new_table as *mut PageTable)).entries[virt_address.vpn0()].0 =
                phys_address.to_ppn() as usize | flags | EntryFlags::Valid as usize;
        }
    }

    pub fn unmap(&mut self) {
        for entry in self.entries.iter_mut() {
            if entry.is_valid() && entry.is_branch() {
                let table = entry.get_phys_address().0 as *mut PageTable;
                unsafe {
                    page::PAGE_ALLOCATOR.lock().dealloc(table as *mut u8);
                }
            }
            entry.0 = 0;
        }
    }

    //Todo: Should this be here? Or in kernel start? If here, whe should make it kernel specific
    pub fn map_kernel_range(&mut self, start: VirtualAddress, end: VirtualAddress, flags: usize) {
        if !start.is_aligned() {
            println!("Start address is not aligned");
            return;
        }

        let start = start.as_usize();
        let aligned_end = page::align_val(end.as_usize(), 12);

        let num_pages = (aligned_end - start) / PAGE_SIZE;
        for i in 0..num_pages {
            self.map(
                VirtualAddress(start).with_offset(i * PAGE_SIZE),
                PhysicalAddress(start as u64).with_offset((i * PAGE_SIZE) as u64),
                flags,
            );
        }
    }

    pub fn print_entries(&self, full: bool) {
        self.print_entries_inner(full, "");
    }

    fn print_entries_inner(&self, full: bool, prefix: &str) {
        for (i, entry) in self.entries.iter().enumerate() {
            if entry.is_valid() {
                //Print the entry, but also the flags of the entry as binary
                println!(
                    "{}Entry {} ({})=> Val: {:#x}, Phys: {:#x} Flags: {:0>10b}",
                    prefix,
                    i,
                    if entry.is_leaf() { "Leaf" } else { "Branch" },
                    entry.0,
                    entry.get_phys_address().0,
                    entry.0 & 0x3FF
                );

                if full && entry.is_branch() {
                    unsafe {
                        let table = entry.get_phys_address().0 as *mut PageTable;
                        (*table).print_entries_inner(full, "\t");
                    }
                }
            }
        }
    }
}

//To satisfy clippy
impl Default for PageTable {
    fn default() -> Self {
        Self::new()
    }
}
