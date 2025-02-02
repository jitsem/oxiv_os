ENTRY(kernel_boot)

SECTIONS {
    . = 0x80200000;

    __kernel_start = .;

    .text : ALIGN(4096){
        __text_start = .;
        KEEP(*(.text.kernel_boot));
        . = ALIGN(16);
        KEEP(*(.text.kernel_entry));
        *(.text .text.*);
        __text_end = .;
    }

    .rodata : ALIGN(4096) {
        __rodata_start = .;
        *(.rodata .rodata.*);
        __rodata_end = .;
    }

    .data : ALIGN(4096) {
        __data_start = .;
        *(.data .data.*);
        __data_end = .;

    }

    .bss : ALIGN(4096) {
        __bss_start = .;
        *(.bss .bss.* .sbss .sbss.*);
        __bss_end = .;
    }


    . = ALIGN(4096);
    __stack_start = .;
    . += 128 * 1024; /* 128KB */
    __stack_end = .;

    . = ALIGN(4096);
    __heap_start = .;
    . += 64 * 1024 * 1024; /* 64MB */
    __heap_end = .;

}
