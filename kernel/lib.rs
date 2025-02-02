#![no_std]

extern crate alloc;
use alloc::vec::Vec;
use allocator::KernelAllocator;
use core::panic::PanicInfo;
use scheduler::Scheduler;

pub mod allocator;
pub mod arch;
pub mod common;
pub mod page;
pub mod page_table;
pub mod process;
pub mod scheduler;
pub mod spinlock;

#[global_allocator]
static mut ALLOCATOR: KernelAllocator = KernelAllocator::new();
static mut ROOT_PAGE_TABLE: page_table::PageTable = page_table::PageTable::new();

pub struct BootInfo {
    pub text_start: usize,
    pub text_end: usize,
    pub rodata_start: usize,
    pub rodata_end: usize,
    pub data_start: usize,
    pub data_end: usize,
    pub bss_start: usize,
    pub bss_end: usize,
    pub stack_start: usize,
    pub stack_end: usize,
    pub heap_start: usize,
    pub heap_end: usize,
}

pub fn boot(boot_info: &BootInfo) {
    arch::init_handlers();

    println!("===============================================");
    println!("      OOOOO   X     X   III  V         V ");
    println!("     O     O   X   X     I    V       V  ");
    println!("     O     O    X X      I     V     V   ");
    println!("     O     O     X       I      V   V    ");
    println!("     O     O    X X      I       V V     ");
    println!("     O     O   X   X     I        V      ");
    println!("      OOOOO   X     X   III       V      ");
    println!("===============================================");
    println!("{}", "Hello World!");
    unsafe {
        init_memory(boot_info);
    }
    println!();
    init_stap(unsafe { &ROOT_PAGE_TABLE as *const _ as usize });
    println!();
    unsafe {
        do_mem_tests();
    }
    println!();
    unsafe {
        init_scheduler();
    }
    println!();
    println!("Kernel initialization done. Going phase 2! Prepare to enter user mode.");
    println!("(Actually not yet, since that's not finised yet. For now just test some process scheduling)");
    println!("===============================================");
    println!();
    unsafe {
        yield_to_init();
    }
}

//TODO: This should also be abstracted away in arch
fn init_stap(addr: usize) {
    let stap = arch::Satp::new(addr);
    println!("Stap: {:x}", stap.get());
    stap.switch();
    println!("Stap register written")
}

// Idea is to have this start the init process. But this is not yet implemented
unsafe fn yield_to_init() {
    println!("Starting process A and B");
    let proc_a_ptr = process_a as *const () as usize;
    let proc_b_ptr = process_b as *const () as usize;
    assert!(
        proc_a_ptr % 2 == 0,
        "proc_a_ptr is not 4-byte aligned: {:#x}",
        proc_a_ptr
    );
    assert!(
        proc_b_ptr % 2 == 0,
        "proc_b_ptr is not 4-byte aligned: {:#x}",
        proc_b_ptr
    );
    println!("proc_a_ptr: {:#X}", proc_a_ptr);
    println!("proc_b_ptr: {:#X}", proc_b_ptr);
    let proc_a = CREATOR.as_mut().unwrap().schedule_process(proc_a_ptr);
    println!("A: {}", proc_a);
    let proc_b = CREATOR.as_mut().unwrap().schedule_process(proc_b_ptr);
    println!("B: {}", proc_b);
    CREATOR.as_mut().unwrap().yield_control();
}

unsafe fn init_scheduler() {
    println!("Initing Scheduler...");
    CREATOR = Some(Scheduler::new());
    CREATOR.as_mut().unwrap().init();
    println!("Scheduler inited!");
}

unsafe fn init_memory(boot_info: &BootInfo) {
    println!("Initiating Page Alloctor: ");
    page::PAGE_ALLOCATOR
        .lock()
        .init(boot_info.heap_start, boot_info.heap_end);
    ALLOCATOR.init(&page::PAGE_ALLOCATOR);
    page::PAGE_ALLOCATOR.lock().print_page_allocations();
    println!();
    println!("Mapping kernel space:");
    println!(
        "TEXT:   0x{:x} -> 0x{:x}",
        boot_info.text_start, boot_info.text_end
    );
    println!(
        "RODATA: 0x{:x} -> 0x{:x}",
        boot_info.rodata_start, boot_info.rodata_end
    );
    println!(
        "DATA:   0x{:x} -> 0x{:x}",
        boot_info.data_start, boot_info.data_end
    );
    println!(
        "BSS:    0x{:x} -> 0x{:x}",
        boot_info.bss_start, boot_info.bss_end
    );
    println!(
        "STACK:  0x{:x} -> 0x{:x}",
        boot_info.stack_start, boot_info.stack_end
    );
    println!(
        "HEAP:   0x{:x} -> 0x{:x}",
        boot_info.heap_start, boot_info.heap_end
    );

    assert!(
        (&ROOT_PAGE_TABLE as *const _ as usize) & 0xFFF == 0,
        "ROOT_PAGE_TABLE is not aligned!"
    );

    ROOT_PAGE_TABLE.map_kernel_range(
        page_table::VirtualAddress(boot_info.text_start),
        page_table::VirtualAddress(boot_info.text_end),
        page_table::EntryFlags::Read as usize | page_table::EntryFlags::Execute as usize,
    );

    ROOT_PAGE_TABLE.map_kernel_range(
        page_table::VirtualAddress(boot_info.rodata_start),
        page_table::VirtualAddress(boot_info.rodata_end),
        page_table::EntryFlags::Read as usize | page_table::EntryFlags::Execute as usize,
    );

    ROOT_PAGE_TABLE.map_kernel_range(
        page_table::VirtualAddress(boot_info.data_start),
        page_table::VirtualAddress(boot_info.data_end),
        page_table::EntryFlags::Read as usize | page_table::EntryFlags::Write as usize,
    );

    ROOT_PAGE_TABLE.map_kernel_range(
        page_table::VirtualAddress(boot_info.bss_start),
        page_table::VirtualAddress(boot_info.bss_end),
        page_table::EntryFlags::Read as usize | page_table::EntryFlags::Write as usize,
    );

    ROOT_PAGE_TABLE.map_kernel_range(
        page_table::VirtualAddress(boot_info.stack_start),
        page_table::VirtualAddress(boot_info.stack_end),
        page_table::EntryFlags::Read as usize | page_table::EntryFlags::Write as usize,
    );
    println!();
    println!("Detailed ROOT_PAGE_TABLE view before heap map:");
    ROOT_PAGE_TABLE.print_entries(false);
    println!();

    ROOT_PAGE_TABLE.map_kernel_range(
        page_table::VirtualAddress(boot_info.heap_start),
        page_table::VirtualAddress(boot_info.heap_end),
        page_table::EntryFlags::Read as usize | page_table::EntryFlags::Write as usize,
    );
    println!("First-level ROOT_PAGE_TABLE view after heap map:");
    ROOT_PAGE_TABLE.print_entries(false);
    println!();
    println!("Mapping kernel space done!");
}

#[allow(static_mut_refs)]
unsafe fn do_mem_tests() {
    println!("Basic memory initialization done! Testing some allocations...");
    //Do some calls to the allocator and print the results
    let page1 = page::PAGE_ALLOCATOR.lock().alloc(10);
    println!("Got {:#x}", page1 as usize);
    page::PAGE_ALLOCATOR.lock().print_page_allocations();
    let page2 = page::PAGE_ALLOCATOR.lock().zero_alloc(10);
    println!("Got {:#x}", page2 as usize);
    page::PAGE_ALLOCATOR.lock().print_page_allocations();
    page::PAGE_ALLOCATOR.lock().dealloc(page1);
    page::PAGE_ALLOCATOR.lock().print_page_allocations();
    page::PAGE_ALLOCATOR.lock().dealloc(page2);
    //DO some rust allocations to test the allocator
    {
        println!("Specifically checking rust global_alloc");
        let mut vec: Vec<usize> = Vec::with_capacity(10);
        vec.push(1);
        vec.push(2);
        vec.push(3);
        vec.push(4);
        vec.push(5);
        page::PAGE_ALLOCATOR.lock().print_page_allocations();
        println!("Vec: {:?}", vec);
        println!("Rust Allocator test done!");
    }
    page::PAGE_ALLOCATOR.lock().print_page_allocations();
    println!("Mem test test done!");
}

/// Since yield_control does not return, we cannot just use our spin-lock to get rid of the static.
/// If we did, we would give control to proc A, yet it would get in the spin when trying to yield itself
/// This will fix itself after we implement a timer interrupt for sheduling.
static mut CREATOR: Option<Scheduler> = None;

#[allow(static_mut_refs)]
unsafe fn process_a() {
    println!("Printing a 3 A's");
    for i in 0..3 {
        println!("A{}", i);
        arch::delay();
        CREATOR.as_mut().unwrap().yield_control();
    }
    println!("A was done!");
    CREATOR.as_mut().unwrap().exit_process();
}
#[allow(static_mut_refs)]
unsafe fn process_b() {
    println!("Printing a 3 B's");
    for i in 0..3 {
        println!("B{}", i);
        arch::delay();
        CREATOR.as_mut().unwrap().yield_control();
    }
    println!("B was done!");
    CREATOR.as_mut().unwrap().exit_process();
}

#[panic_handler]
fn handle_panic(info: &PanicInfo) -> ! {
    print!("Kernel Panic");
    if let Some(location) = info.location() {
        print!(" ({},{})", location.line(), location.column())
    }
    print!(": {}", info.message());
    arch::abort();
}
