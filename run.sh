#!/bin/bash
set -xue

# QEMU file path
QEMU=qemu-system-riscv32

if [[ $# -gt 0 && "$1" == "--log" ]]; then
    $QEMU -machine virt -bios default -nographic -serial mon:stdio -no-reboot -d unimp,guest_errors,int,cpu_reset -D qemu.log -kernel target/riscv32imac-unknown-none-elf/release/oxiv_os
else
    $QEMU -machine virt -bios default -nographic -serial mon:stdio -no-reboot -kernel target/riscv32imac-unknown-none-elf/release/oxiv_os
fi
