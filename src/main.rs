#![no_std]
#![no_main]

extern crate alloc;
use core::arch::asm;
use core::panic::PanicInfo;

use allocator::KernelAllocator;
use page::PageAllocator;
use scheduler::Scheduler;
use spinlock::SpinLock;

pub mod allocator;
pub mod common;
pub mod page;
pub mod process;
pub mod sbi;
pub mod scheduler;
pub mod spinlock;

//These are "filled in" by the linker
extern "C" {
    static __stack_top: *const usize;
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
            stack_top = sym __stack_top,
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
static mut PAGE_ALLOCATOR: SpinLock<page::PageAllocator> = SpinLock::new(PageAllocator::new());

/// Main function
///
/// This does the actual init of the kernel functions, as opposed to the kernel_boot which does the mandatory asm stuff
///
/// # Safety
/// - This function expects valid adresses for __heap_start and __heap_end
#[no_mangle]
#[allow(static_mut_refs)]
unsafe fn main() {
    write_stvec(kernel_entry as *const () as usize, StvecMode::Direct);
    println!("{}", "Hello World!");
    println!("Initiating our page allocator and running some tests");

    PAGE_ALLOCATOR.lock().init(
        &__heap_start as *const _ as usize,
        &__heap_end as *const _ as usize,
    );
    PAGE_ALLOCATOR.lock().print_page_allocations();
    //Do some calls to the allocator and print the results
    let page1 = PAGE_ALLOCATOR.lock().alloc(10);
    println!("Got {:#x}", page1 as usize);
    PAGE_ALLOCATOR.lock().print_page_allocations();
    let page2 = PAGE_ALLOCATOR.lock().zero_alloc(10);
    println!("Got {:#x}", page2 as usize);
    PAGE_ALLOCATOR.lock().print_page_allocations();
    PAGE_ALLOCATOR.lock().dealloc(page1);
    PAGE_ALLOCATOR.lock().print_page_allocations();
    PAGE_ALLOCATOR.lock().dealloc(page2);
    println!("Coupling to the rust allocator",);
    ALLOCATOR.init(&PAGE_ALLOCATOR);

    println!("Starting a process");
    CREATOR = Some(Scheduler::new());
    CREATOR.as_mut().unwrap().init();
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
