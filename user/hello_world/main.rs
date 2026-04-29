#![no_std]
#![no_main]

use core::arch::asm;
use core::panic::PanicInfo;

/// The entry point our app
/// # Safety
/// - This function must only be called during the app initialization phase.
/// - `main` must be a valid function symbol with a proper ABI.
/// - `exit` must be a valid function symbol with a proper ABI.
#[link_section = ".text.app_boot"]
#[no_mangle]
pub unsafe extern "C" fn app_boot() {
    unsafe {
        asm!(
            "call {main}",
            "call {exit}",
            main = sym main,
            exit = sym exit,
            options(noreturn),
        );
    }
}

fn main() {
    for _ in 0..100 {
        yield_cpu();
    }
}

fn exit() -> ! {
    unsafe {
        asm!("li a7, 1", "ecall", options(noreturn));
    }
}

fn yield_cpu() {
    unsafe {
        asm!("li a7, 0", "ecall", options(nostack));
    }
}

#[panic_handler]
fn handle_panic(_info: &PanicInfo) -> ! {
    loop {}
}
