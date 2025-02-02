#![no_std]
#![no_main]

extern crate alloc;
use alloc::vec::Vec;
use allocator::KernelAllocator;
use core::arch::asm;
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

//These are "filled in" by the linker
extern "C" {
    static __text_start: *const usize;
    static __text_end: *const usize;

    static __rodata_start: *const usize;
    static __rodata_end: *const usize;

    static __data_start: *const usize;
    static __data_end: *const usize;

    static __bss_start: *const usize;
    static __bss_end: *const usize;

    static __stack_start: *const usize;
    static __stack_end: *const usize;

    static __heap_start: *const usize;
    static __heap_end: *const usize;
}

/// Boot Entry point of our kernel.
///
/// Sets the correct address in the stack pointer and jumps to the main function.
///
/// # Safety
/// - This function must only be called during the kernel initialization phase.
/// - `__stack_top` must point to a valid stack memory region.
/// - `main` must be a valid function symbol with a proper ABI.
#[link_section = ".text.kernel_boot"]
#[no_mangle]
pub unsafe extern "C" fn kernel_boot() -> ! {
    unsafe {
        asm!(
            "la sp, {stack_top}",
            "j {main}",
            stack_top = sym __stack_end,
            main = sym main,
            options(noreturn),
        );
    }
}

#[global_allocator]
static mut ALLOCATOR: KernelAllocator = KernelAllocator::new();
static mut ROOT_PAGE_TABLE: page_table::PageTable = page_table::PageTable::new();

fn convert_ptr_to_usize<T>(ptr: &*const T) -> usize {
    ptr as *const _ as usize
}

/// Main function
///
/// This does the actual init of the kernel functions, as opposed to the kernel_boot which does the mandatory asm stuff
///
/// # Safety
/// - This function expects valid adresses for __heap_start and __heap_end
#[no_mangle]
#[allow(static_mut_refs)]
unsafe fn main() {
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
    init_memory();
    println!();
    init_stap(&ROOT_PAGE_TABLE as *const _ as usize);
    println!();
    do_mem_tests();
    println!();
    init_scheduler();
    println!();
    println!("Kernel initialization done. Going phase 2! Prepare to enter user mode.");
    println!("(Actually not yet, since that's not finised yet.");
    println!("===============================================");
    println!();

    yield_to_init();
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

#[allow(static_mut_refs)]
unsafe fn init_memory() {
    let text_start = convert_ptr_to_usize(&__text_start);
    let text_end = convert_ptr_to_usize(&__text_end);
    let rodata_start = convert_ptr_to_usize(&__rodata_start);
    let rodata_end = convert_ptr_to_usize(&__rodata_end);
    let data_start = convert_ptr_to_usize(&__data_start);
    let data_end = convert_ptr_to_usize(&__data_end);
    let bss_start = convert_ptr_to_usize(&__bss_start);
    let bss_end = convert_ptr_to_usize(&__bss_end);
    let stack_start = convert_ptr_to_usize(&__stack_start);
    let stack_end = convert_ptr_to_usize(&__stack_end);
    let heap_start = convert_ptr_to_usize(&__heap_start);
    let heap_end = convert_ptr_to_usize(&__heap_end);
    println!("Initiating Page Alloctor: ");
    page::PAGE_ALLOCATOR.lock().init(heap_start, heap_end);
    ALLOCATOR.init(&page::PAGE_ALLOCATOR);
    page::PAGE_ALLOCATOR.lock().print_page_allocations();
    println!();
    println!("Mapping kernel space:");
    println!("TEXT:   0x{:x} -> 0x{:x}", text_start, text_end);
    println!("RODATA: 0x{:x} -> 0x{:x}", rodata_start, rodata_end);
    println!("DATA:   0x{:x} -> 0x{:x}", data_start, data_end);
    println!("BSS:    0x{:x} -> 0x{:x}", bss_start, bss_end);
    println!("STACK:  0x{:x} -> 0x{:x}", stack_start, stack_end);
    println!("HEAP:   0x{:x} -> 0x{:x}", heap_start, heap_end);

    assert!(
        (&ROOT_PAGE_TABLE as *const _ as usize) & 0xFFF == 0,
        "ROOT_PAGE_TABLE is not aligned!"
    );

    ROOT_PAGE_TABLE.map_kernel_range(
        page_table::VirtualAddress(text_start),
        page_table::VirtualAddress(text_end),
        page_table::EntryFlags::Read as usize | page_table::EntryFlags::Execute as usize,
    );

    ROOT_PAGE_TABLE.map_kernel_range(
        page_table::VirtualAddress(rodata_start),
        page_table::VirtualAddress(rodata_end),
        page_table::EntryFlags::Read as usize | page_table::EntryFlags::Execute as usize,
    );

    ROOT_PAGE_TABLE.map_kernel_range(
        page_table::VirtualAddress(data_start),
        page_table::VirtualAddress(data_end),
        page_table::EntryFlags::Read as usize | page_table::EntryFlags::Write as usize,
    );

    ROOT_PAGE_TABLE.map_kernel_range(
        page_table::VirtualAddress(bss_start),
        page_table::VirtualAddress(bss_end),
        page_table::EntryFlags::Read as usize | page_table::EntryFlags::Write as usize,
    );

    ROOT_PAGE_TABLE.map_kernel_range(
        page_table::VirtualAddress(stack_start),
        page_table::VirtualAddress(stack_end),
        page_table::EntryFlags::Read as usize | page_table::EntryFlags::Write as usize,
    );
    println!();
    println!("Detailed ROOT_PAGE_TABLE view before heap map:");
    ROOT_PAGE_TABLE.print_entries(false);
    println!();

    ROOT_PAGE_TABLE.map_kernel_range(
        page_table::VirtualAddress(heap_start),
        page_table::VirtualAddress(heap_end),
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
