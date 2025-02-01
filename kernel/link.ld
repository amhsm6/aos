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

ENTRY(_start)

SECTIONS {
    _kernel_begin = 0xffff800000000000;
    _kernel_size = 512M;
    _stack_size = 1M;

    . = _kernel_begin;
    .text : AT(0) {
        *(.text*)
    }

    .data : {
        *(.data*)
    }

    .rodata : {
        *(.rodata*)
    }

    .got : {
        *(.got*)
    }

    .bss : {
        *(.bss*)
    }

    . = _kernel_begin + _kernel_size;
    .stack : AT(_kernel_size) {
        _stack_bottom = .;
        . += _stack_size;
        _stack_top = .;
    }

    /DISCARD/ : { *(.eh_frame*) *(.comment*) }
}
