#![no_std]
#![no_main]

use core::arch::asm;
use core::panic::PanicInfo;

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

#[link_section = ".text.app_boot"]
#[no_mangle]
pub unsafe extern "C" fn app_boot() -> () {
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
fn main() {
    let mut i = 0;
    loop {
        i += 1
    }
}

#[panic_handler]
fn handle_panic(info: &PanicInfo) -> ! {
    loop {}
}
