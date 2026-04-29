use std::{env, fs, path::PathBuf, process::Command};

fn main() {
    // hello_world must be built before this build script runs.
    // Run `./run.sh` or `cargo build -p hello_world --target riscv32imac-unknown-none-elf --release`
    // before building the kernel. See run.sh for the correct build order.
    run(
        "llvm-objcopy",
        &[
            "--set-section-flags",
            ".bss=alloc,contents",
            "-O",
            "binary",
            "../../target/riscv32imac-unknown-none-elf/release/hello_world",
            "../../target/riscv32imac-unknown-none-elf/release/hello_world.bin",
        ],
    );

    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let dst = out_dir.join("hello_world.bin");
    fs::copy(
        "../../target/riscv32imac-unknown-none-elf/release/hello_world.bin",
        &dst,
    )
    .expect("copy failed: build hello_world first (see run.sh)");

    println!(
        "cargo:rerun-if-changed=../../target/riscv32imac-unknown-none-elf/release/hello_world.bin"
    );
    println!("cargo:rustc-link-arg=-Tboot/riscv32/script.ld");
    println!("cargo:rustc-link-arg=--omagic");
}

fn run(cmd: &str, args: &[&str]) {
    let status = Command::new(cmd)
        .args(args)
        .status()
        .expect("failed to run command");
    if !status.success() {
        panic!("{cmd} failed with status {status}");
    }
}
