/*

0xffffffffaf000000
|
| KERNEL (512MB)
|
0xffffffffcf000000
|
| STACK (1MB)
|
0xffffffffcf100000

*/

ENTRY(_start)

SECTIONS {
    _kernel_begin = 0xffffffffaf000000;
    _kernel_size = 512M;
    _stack_size = 1M;

    . = _kernel_begin;
    .text : AT(0) {
        *(.ltext*)
    }

    .data : {
        *(.ldata*)
    }

    .rodata : {
        *(.lrodata*)
    }

    .bss : {
        *(.lbss*)
    }

    . = _kernel_begin + _kernel_size;
    .stack : AT(_kernel_size) {
        _stack_bottom = .;
        . += _stack_size;
        _stack_top = .;
    }

    /DISCARD/ : { *(.eh_frame*) *(.comment*) }
}
