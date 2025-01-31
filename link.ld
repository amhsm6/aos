SECTIONS {
    . = ALIGN(4096);
    .text : AT(0x0) {
        *(.text*)
    }

    . = ALIGN(4096);
    .data : {
        *(.data*)
    }

    . = ALIGN(4096);
    .rodata : {
        *(.rodata*)
    }

    . = ALIGN(4096);
    .got : {
        *(.got*)
    }

    . = ALIGN(4096);
    .bss : {
        *(.bss*)
    }

    /DISCARD/ : { *(.eh_frame*) *(.comment) }
}
