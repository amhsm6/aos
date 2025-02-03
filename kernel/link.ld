/*

0xffffffffaf000000
|
| KERNEL (512MB)
|
0xffffffffceffffff
|
*
|
0xfffffffffff00000
|
| STACK (1MB)
|
0xffffffffffffffff

*/

ENTRY(_start)

SECTIONS {
    _kernel_begin = 0xffffffffaf000000;
    _kernel_size = 512M;

    _stack_begin = 0xfffffffffff00000;
    _stack_size = 1M;

    . = _kernel_begin;
    .text : AT(0) {
        KEEP(*(.ltext.astart))

        _start = .;
        KEEP(*(.ltext._start))

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
    _kernel_end = .;

    . = _stack_begin;
    .stack : AT(_stack_begin - _kernel_begin) {
        _stack_bottom = .;
        . += _stack_size - 1;
        _stack_top = .;
    }
    _stack_end = .;

    /DISCARD/ : { *(.eh_frame*) *(.comment*) }
}

ASSERT(_kernel_end - _kernel_begin + 1 <= _kernel_size, "ERROR: Kernel exceeding size limit");
ASSERT(_stack_end - _stack_begin + 1 <= _stack_size, "ERROR: Stack exceeding size limit");
