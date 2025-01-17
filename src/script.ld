ENTRY(kernel_boot)

SECTIONS {
    . = 0x80200000;

    __kernel_start = .;

    .text : ALIGN(16){
        KEEP(*(.text.kernel_boot));
        . = ALIGN(16);
        KEEP(*(.text.kernel_entry));
        *(.text .text.*);
    }

    .rodata : ALIGN(4) {
        *(.rodata .rodata.*);
    }

    .data : ALIGN(4) {
        *(.data .data.*);
    }

    .bss : ALIGN(4) {
        __bss = .;
        *(.bss .bss.* .sbss .sbss.*);
        __bss_end = .;
    }


    . = ALIGN(16);
    . += 128 * 1024; /* 128KB */
    __stack_top = .;

}
