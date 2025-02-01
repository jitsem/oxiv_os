#![no_std]
#![no_main]

extern crate alloc;
use alloc::vec::Vec;
use allocator::KernelAllocator;
use core::arch::asm;
use core::arch::global_asm;
use core::panic::PanicInfo;
use scheduler::Scheduler;

pub mod allocator;
pub mod common;
pub mod page;
pub mod page_table;
pub mod process;
pub mod sbi;
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

/// Trap handler entry point of our kernel.
///
/// Stores the current program state in registers. Calls the actual trap handler and restores the state
/// # Safety
/// - This function must only be called during the kernel trap phase.
/// - `handle_trap` must be a valid function symbol with a proper ABI.
#[no_mangle]
#[link_section = ".text.kernel_entry"]
pub extern "C" fn kernel_entry() {
    unsafe {
        asm!(
            "csrw sscratch, sp",
            "addi sp, sp, -4 * 31",
            "sw ra,  4 * 0(sp)",
            "sw gp,  4 * 1(sp)",
            "sw tp,  4 * 2(sp)",
            "sw t0,  4 * 3(sp)",
            "sw t1,  4 * 4(sp)",
            "sw t2,  4 * 5(sp)",
            "sw t3,  4 * 6(sp)",
            "sw t4,  4 * 7(sp)",
            "sw t5,  4 * 8(sp)",
            "sw t6,  4 * 9(sp)",
            "sw a0,  4 * 10(sp)",
            "sw a1,  4 * 11(sp)",
            "sw a2,  4 * 12(sp)",
            "sw a3,  4 * 13(sp)",
            "sw a4,  4 * 14(sp)",
            "sw a5,  4 * 15(sp)",
            "sw a6,  4 * 16(sp)",
            "sw a7,  4 * 17(sp)",
            "sw s0,  4 * 18(sp)",
            "sw s1,  4 * 19(sp)",
            "sw s2,  4 * 20(sp)",
            "sw s3,  4 * 21(sp)",
            "sw s4,  4 * 22(sp)",
            "sw s5,  4 * 23(sp)",
            "sw s6,  4 * 24(sp)",
            "sw s7,  4 * 25(sp)",
            "sw s8,  4 * 26(sp)",
            "sw s9,  4 * 27(sp)",
            "sw s10, 4 * 28(sp)",
            "sw s11, 4 * 29(sp)",
            "csrr a0, sscratch",
            "sw a0, 4 * 30(sp)",
            "mv a0, sp",
            "call {handle_trap}",
            "lw ra,  4 * 0(sp)",
            "lw gp,  4 * 1(sp)",
            "lw tp,  4 * 2(sp)",
            "lw t0,  4 * 3(sp)",
            "lw t1,  4 * 4(sp)",
            "lw t2,  4 * 5(sp)",
            "lw t3,  4 * 6(sp)",
            "lw t4,  4 * 7(sp)",
            "lw t5,  4 * 8(sp)",
            "lw t6,  4 * 9(sp)",
            "lw a0,  4 * 10(sp)",
            "lw a1,  4 * 11(sp)",
            "lw a2,  4 * 12(sp)",
            "lw a3,  4 * 13(sp)",
            "lw a4,  4 * 14(sp)",
            "lw a5,  4 * 15(sp)",
            "lw a6,  4 * 16(sp)",
            "lw a7,  4 * 17(sp)",
            "lw s0,  4 * 18(sp)",
            "lw s1,  4 * 19(sp)",
            "lw s2,  4 * 20(sp)",
            "lw s3,  4 * 21(sp)",
            "lw s4,  4 * 22(sp)",
            "lw s5,  4 * 23(sp)",
            "lw s6,  4 * 24(sp)",
            "lw s7,  4 * 25(sp)",
            "lw s8,  4 * 26(sp)",
            "lw s9,  4 * 27(sp)",
            "lw s10, 4 * 28(sp)",
            "lw s11, 4 * 29(sp)",
            "lw sp,  4 * 30(sp)",
            "sret",
            handle_trap = sym handle_trap,
        );
    }
}

pub fn delay() {
    for _ in 0..30000 {
        unsafe {
            asm!("nop");
        }
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
    register_cpu_handlers();

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
    init_stap();
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
unsafe fn init_stap() {
    // STAP register is [MODE: 1 bit][ASID: 9 bits][PPN: 22 bits]. For now I don't care about the ASID. I think??
    // So we OR 1 in the top bit with the root page table address divided by the page size to get the physical page adress
    let satp = 1usize << 31
        | (&ROOT_PAGE_TABLE as *const page_table::PageTable as usize >> page::PAGE_ORDER);
    println!("satp: {:#x}", satp);
    __set_satp(satp);
    println!("satp set!");
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

fn register_cpu_handlers() {
    // Set trap handler
    write_stvec(kernel_entry as *const () as usize, StvecMode::Direct);
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
        delay();
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
        delay();
        CREATOR.as_mut().unwrap().yield_control();
    }
    println!("B was done!");
    CREATOR.as_mut().unwrap().exit_process();
}

#[no_mangle]
fn handle_trap(frame: &TrapFrame) {
    let scause = read_csr("scause");
    let stval = read_csr("stval");
    let spec = read_csr("sepc");
    let sp = frame.sp;
    panic!(
        "unexpected trap scause={:#x}, stval={:#x}, sepc={:#x}, sp={:#x}\n",
        scause, stval, spec, sp
    );
}

#[repr(C, packed)]
pub struct TrapFrame {
    ra: usize,
    gp: usize,
    tp: usize,
    t0: usize,
    t1: usize,
    t2: usize,
    t3: usize,
    t4: usize,
    t5: usize,
    t6: usize,
    a0: usize,
    a1: usize,
    a2: usize,
    a3: usize,
    a4: usize,
    a5: usize,
    a6: usize,
    a7: usize,
    s0: usize,
    s1: usize,
    s2: usize,
    s3: usize,
    s4: usize,
    s5: usize,
    s6: usize,
    s7: usize,
    s8: usize,
    s9: usize,
    s10: usize,
    s11: usize,
    sp: usize,
}

fn read_csr(reg: &str) -> usize {
    let value: usize;
    unsafe {
        match reg {
            "scause" => asm!("csrr {0}, scause", out(reg) value),
            "stval" => asm!("csrr {0}, stval", out(reg) value),
            "sepc" => asm!("csrr {0}, sepc", out(reg) value),
            _ => panic!("Unsupported CSR: {}", reg),
        }
    }
    value
}

#[repr(usize)]
pub enum StvecMode {
    Direct = 0,
}

pub fn write_stvec(addr: usize, mode: StvecMode) {
    unsafe {
        asm!("csrw stvec, {}", in(reg) (addr | mode as usize));
    }
}

#[panic_handler]
fn handle_panic(info: &PanicInfo) -> ! {
    print!("Kernel Panic");
    if let Some(location) = info.location() {
        print!(" ({},{})", location.line(), location.column())
    }
    print!(": {}", info.message());
    abort();
}

#[no_mangle]
extern "C" fn abort() -> ! {
    loop {
        unsafe {
            asm!("wfi");
        }
    }
}

extern "C" {
    fn __set_satp(satp: usize);
}
global_asm!("__set_satp:", "csrw satp, a0", "sfence.vma", "ret");
