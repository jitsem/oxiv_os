#!/bin/bash
set -xue

# QEMU file path
QEMU=qemu-system-riscv32

# User programs must be built before the kernel (build.rs embeds the binary).
# Cargo can't handle this ordering automatically without a dependency, so we do it here.
cargo build --release -p hello_world
cargo build --release

if [[ $# -gt 0 && "$1" == "--log" ]]; then
    $QEMU -machine virt -bios default -nographic -serial mon:stdio -no-reboot -d unimp,guest_errors,int,cpu_reset -D qemu.log -kernel target/riscv32imac-unknown-none-elf/release/oxiv_riscv32
else
    $QEMU -machine virt -bios default -nographic -serial mon:stdio -no-reboot -kernel target/riscv32imac-unknown-none-elf/release/oxiv_riscv32
fi
