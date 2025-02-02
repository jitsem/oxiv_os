fn main() {
    // Use the linker script.
    println!("cargo:rustc-link-arg=-Tboot/riscv32/script.ld");
    // Don't do any magic linker stuff.
    println!("cargo:rustc-link-arg=--omagic");
}
