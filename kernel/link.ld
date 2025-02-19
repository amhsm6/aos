/*

0xfffffffe_00000000
|
| ACPI
|
0xffffffff_00000000
|
| PCI
|
*
|
0xffffffff_af000000
|
| KERNEL (512MB)
|
0xffffffff_cf000000
|
*
|
0xffffffff_dff00000
|
| HEAP (512MB)
|
0xffffffff_fff00000
|
| STACK (~1MB)
|
0xffffffff_fffffff0

*/

ENTRY(kstart)

SECTIONS {
    _kernel_begin = 0xffffffffaf000000;
    _kernel_size = 512M;

    _heap_begin = 0xffffffffdff00000;
    _heap_size = 512M;

    _stack_begin = 0xfffffffffff00000;
    _stack_end = 0xfffffffffffffff0;
    _stack_size = _stack_end - _stack_begin;

    . = _kernel_begin;
    .text : AT(0) {
        KEEP(*(.ltext.astart))

        kstart = .;
        KEEP(*(.ltext.kstart))

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

    . = _heap_begin;
    .heap (NOLOAD) : AT(_heap_begin - _kernel_begin) {
        . += _heap_size;
    }
    _heap_end = .;

    . = _stack_begin;
    .stack (NOLOAD) : AT(_stack_begin - _kernel_begin) {
        _stack_bottom = .;
        . += _stack_size;
        _stack_top = .;
    }
    _stack_end = .;

    /DISCARD/ : { *(.eh_frame*) *(.comment*) }
}

ASSERT(_kernel_end - _kernel_begin <= _kernel_size, "ERROR: Kernel exceeding size limit");
