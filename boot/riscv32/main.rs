#![no_std]
#![no_main]
use core::arch::asm;
use oxiv_kernel::{boot, BootInfo};
// Based myself on https://github.com/starina-os/starina for the boot (specific) -> kernel (generic) -> arch (specific) structure
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

#[no_mangle]
fn main() {
    let boot_info = unsafe {
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
        BootInfo {
            text_start,
            text_end,
            rodata_start,
            rodata_end,
            data_start,
            data_end,
            bss_start,
            bss_end,
            stack_start,
            stack_end,
            heap_start,
            heap_end,
        }
    };
    boot(&boot_info);
}

fn convert_ptr_to_usize<T>(ptr: &*const T) -> usize {
    ptr as *const _ as usize
}
