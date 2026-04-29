#![no_std]

extern crate alloc;
use crate::page_table::PageTable;
use alloc::vec::Vec;
use allocator::KernelAllocator;
use core::arch::asm;
use core::panic::PanicInfo;
use process::ProcessState;
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
static ALLOCATOR: KernelAllocator = KernelAllocator;

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
    pub userprog_start: usize,
    pub userprog_end: usize,
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
    let root_page_table = unsafe { init_memory(boot_info) };
    println!();
    init_stap(&root_page_table as *const _ as usize);
    println!();
    unsafe {
        do_mem_tests();
    }
    println!();
    unsafe {
        init_scheduler(&root_page_table as *const _);
    }
    println!();
    println!("Kernel initialization done. Entering U-mode.");
    println!("===============================================");
    println!();
    unsafe {
        yield_to_init(
            &root_page_table as *const _,
            boot_info.userprog_start,
            boot_info.userprog_end,
        );
    }
}

fn init_stap(addr: usize) {
    let stap = arch::Satp::new(addr);
    println!("Stap: {:x}", stap.get());
    stap.switch();
    println!("Stap register written")
}

// Virtual address constants for the user address space
const USER_STACK_TOP: usize = 0x7FFF_F000; // just below 2GB kernel boundary
const USER_STACK_SIZE: usize = 128 * 1024; // 128 KB

/// Kernel trap handler — called from kernel_entry assembly with the saved TrapFrame.
/// Returns the TrapFrame pointer to restore from (may be a different process).
#[no_mangle]
#[allow(static_mut_refs)]
unsafe extern "C" fn handle_trap(frame: *mut arch::TrapFrame) -> *mut arch::TrapFrame {
    let scause: usize;
    asm!("csrr {}, scause", out(reg) scause);
    match scause {
        8 => {
            // ecall from U-mode: advance sepc past the ecall instruction
            (*frame).sepc += 4;
            match (*frame).a7 {
                0 => {
                    // yield
                    let next = CREATOR
                        .as_mut()
                        .unwrap()
                        .prepare_next_process(frame as usize);
                    next as *mut arch::TrapFrame
                }
                1 => {
                    // exit
                    CREATOR
                        .as_mut()
                        .unwrap()
                        .current_running
                        .as_mut()
                        .unwrap()
                        .state = ProcessState::Exited;
                    let next = CREATOR
                        .as_mut()
                        .unwrap()
                        .prepare_next_process(frame as usize);
                    next as *mut arch::TrapFrame
                }
                n => {
                    panic!("unknown syscall a7={}", n);
                }
            }
        }
        _ => {
            let stval: usize;
            asm!("csrr {}, stval", out(reg) stval);
            panic!(
                "unexpected trap scause={:#x}, sepc={:#x}, stval={:#x}, sp={:#x}",
                scause,
                (*frame).sepc,
                stval,
                (*frame).sp,
            );
        }
    }
}

#[allow(static_mut_refs)]
unsafe fn yield_to_init(
    kernel_page_table: *const PageTable,
    userprog_start: usize,
    userprog_end: usize,
) {
    println!("Starting user processes A and B");
    println!("User binary: {:#x} - {:#x}", userprog_start, userprog_end);

    let (pt_a, user_stack_top) =
        create_user_page_table(kernel_page_table, userprog_start, userprog_end);
    let (pt_b, _) = create_user_page_table(kernel_page_table, userprog_start, userprog_end);

    let proc_a = CREATOR
        .as_mut()
        .unwrap()
        .schedule_user_process(0x0100_0000, pt_a, user_stack_top);
    println!("A: {}", proc_a);
    let proc_b = CREATOR
        .as_mut()
        .unwrap()
        .schedule_user_process(0x0100_0000, pt_b, user_stack_top);
    println!("B: {}", proc_b);

    // Pop the first process and make it current, then sret into U-mode.
    // All subsequent scheduling goes through handle_trap.
    let first = CREATOR.as_mut().unwrap().processes.pop_front().unwrap();
    let frame_ptr = first.context.sp;
    let kernel_sp_top = first.kernel_sp_top;
    let page_table = first.page_table_addr;
    CREATOR.as_mut().unwrap().current_running = Some(first);

    println!("Entering U-mode (process A)");

    // Set sscratch = kernel stack top so the first trap swaps correctly
    asm!("csrw sscratch, {}", in(reg) kernel_sp_top);

    // Switch to user page table
    arch::Satp::new(page_table).switch();

    // Restore the fake TrapFrame and sret into U-mode — never returns
    arch::restore_user_and_sret(frame_ptr as *mut arch::TrapFrame);
}

unsafe fn create_user_page_table(
    kernel_pt: *const PageTable,
    userprog_phys_start: usize,
    userprog_phys_end: usize,
) -> (usize, usize) {
    // Allocate and populate a root page table for a U-mode process.
    let page_ptr = page::PAGE_ALLOCATOR.lock().zero_alloc(1);
    assert!(!page_ptr.is_null(), "Failed to allocate user page table");

    // Shallow-copy kernel entries (shares second-level tables for kernel space).
    core::ptr::write(page_ptr as *mut PageTable, (*kernel_pt).clone());
    let user_pt = &mut *(page_ptr as *mut PageTable);

    // Map user binary at virtual 0x1000000 with R/X/User flags
    let binary_size = userprog_phys_end - userprog_phys_start;
    user_pt.map_range(
        0x0100_0000,
        0x0100_0000 + binary_size,
        userprog_phys_start,
        page_table::EntryFlags::Read as usize
            | page_table::EntryFlags::Execute as usize
            | page_table::EntryFlags::User as usize,
    );

    // Allocate and map user stack at USER_STACK_TOP with R/W/User flags
    let stack_phys = page::PAGE_ALLOCATOR
        .lock()
        .zero_alloc(USER_STACK_SIZE / arch::PAGE_SIZE);
    assert!(!stack_phys.is_null(), "Failed to allocate user stack");
    user_pt.map_range(
        USER_STACK_TOP - USER_STACK_SIZE,
        USER_STACK_TOP,
        stack_phys as usize,
        page_table::EntryFlags::Read as usize
            | page_table::EntryFlags::Write as usize
            | page_table::EntryFlags::User as usize,
    );

    (page_ptr as usize, USER_STACK_TOP)
}

#[allow(static_mut_refs)]
unsafe fn init_scheduler(page_table_addr: *const PageTable) {
    println!("Initing Scheduler...");
    CREATOR = Some(Scheduler::new());
    CREATOR.as_mut().unwrap().init(page_table_addr);
    println!("Scheduler inited!");
}

unsafe fn init_memory(boot_info: &BootInfo) -> PageTable {
    println!("Initiating Page Alloctor: ");
    page::PAGE_ALLOCATOR
        .lock()
        .init(boot_info.heap_start, boot_info.heap_end);
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

    let mut root_page = PageTable::new();
    assert_eq!(
        (&(root_page) as *const _ as usize) & 0xFFF,
        0,
        "ROOT_PAGE_TABLE is not aligned!"
    );

    root_page.map_kernel_range(
        page_table::VirtualAddress(boot_info.text_start),
        page_table::VirtualAddress(boot_info.text_end),
        page_table::EntryFlags::Read as usize | page_table::EntryFlags::Execute as usize,
    );
    root_page.map_kernel_range(
        page_table::VirtualAddress(boot_info.rodata_start),
        page_table::VirtualAddress(boot_info.rodata_end),
        page_table::EntryFlags::Read as usize | page_table::EntryFlags::Execute as usize,
    );
    root_page.map_kernel_range(
        page_table::VirtualAddress(boot_info.data_start),
        page_table::VirtualAddress(boot_info.data_end),
        page_table::EntryFlags::Read as usize | page_table::EntryFlags::Write as usize,
    );
    root_page.map_kernel_range(
        page_table::VirtualAddress(boot_info.bss_start),
        page_table::VirtualAddress(boot_info.bss_end),
        page_table::EntryFlags::Read as usize | page_table::EntryFlags::Write as usize,
    );
    root_page.map_kernel_range(
        page_table::VirtualAddress(boot_info.stack_start),
        page_table::VirtualAddress(boot_info.stack_end),
        page_table::EntryFlags::Read as usize | page_table::EntryFlags::Write as usize,
    );
    println!();
    println!("Detailed ROOT_PAGE_TABLE view before heap map:");
    root_page.print_entries(false);
    println!();

    root_page.map_kernel_range(
        page_table::VirtualAddress(boot_info.heap_start),
        page_table::VirtualAddress(boot_info.heap_end),
        page_table::EntryFlags::Read as usize | page_table::EntryFlags::Write as usize,
    );
    println!("First-level ROOT_PAGE_TABLE view after heap map:");
    root_page.print_entries(false);
    println!();
    println!("Mapping kernel space done!");
    root_page
}

#[allow(static_mut_refs)]
unsafe fn do_mem_tests() {
    println!("Basic memory initialization done! Testing some allocations...");
    let page1 = page::PAGE_ALLOCATOR.lock().alloc(10);
    println!("Got {:#x}", page1 as usize);
    page::PAGE_ALLOCATOR.lock().print_page_allocations();
    let page2 = page::PAGE_ALLOCATOR.lock().zero_alloc(10);
    println!("Got {:#x}", page2 as usize);
    page::PAGE_ALLOCATOR.lock().print_page_allocations();
    page::PAGE_ALLOCATOR.lock().dealloc(page1);
    page::PAGE_ALLOCATOR.lock().print_page_allocations();
    page::PAGE_ALLOCATOR.lock().dealloc(page2);
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

static mut CREATOR: Option<Scheduler> = None;

#[panic_handler]
fn handle_panic(info: &PanicInfo) -> ! {
    print!("Kernel Panic");
    if let Some(location) = info.location() {
        print!(" ({},{})", location.line(), location.column())
    }
    print!(": {}", info.message());
    arch::abort();
}
