fn main() {
    // Use the linker script.
    println!("cargo:rustc-link-arg=-Tuser/hello_world/script.ld");
    // Don't do any magic linker stuff.
    println!("cargo:rustc-link-arg=--omagic");
}
