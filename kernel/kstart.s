    .intel_syntax noprefix

    mov rax, cr3
    mov rbx, 0b11111111111
    and rbx, rax

    mov rax, rdi
    mov rdi, rsi
    mov rsi, rdx

    or rax, rbx
    mov cr3, rax

    mov rsp, 0xffffffffffffffff
    mov rax, 0xffffffffaf000000
    jmp rax
