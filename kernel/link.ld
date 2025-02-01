/*

0xffff8000_00000000
|
| KERNEL (512MB)
|
0xffff8000_20000000
|
| STACK (1MB)
|
0xffff8000_20100000

*/

MEMORY {
    KERNEL : ORIGIN = 0x00000000, LENGTH = 512M
    STACK  : ORIGIN = 0x20000000, LENGTH = 1M
}

ENTRY(_start)

SECTIONS {
    .text : {
        *(.text*)
    } > KERNEL

    .data : {
        *(.data*)
    } > KERNEL

    .rodata : {
        *(.rodata*)
    } > KERNEL

    .got : {
        *(.got*)
    } > KERNEL

    .bss : {
        *(.bss*)
    } > KERNEL

    .stack : AT(ADDR(.bss) + SIZEOF(.bss)) {
        _stack_bottom = .;
        . += LENGTH(STACK);
        _stack_top = .;
    } > STACK

    /DISCARD/ : { *(.eh_frame*) *(.comment*) }
}
