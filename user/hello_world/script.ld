ENTRY(app_boot)

SECTIONS {
    . = 0x01000000;

    __app_start = .;

    .text : ALIGN(4096){
        __text_start = .;
        KEEP(*(.text.app_boot));
        . = ALIGN(16);
        KEEP(*(.text.app_entry));
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

    /* Fixed virtual address for the kernel exit trampoline mapped into user page tables */
    PROVIDE(__user_exit_fn = 0x00003000);
}
