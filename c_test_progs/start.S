.section .text.entry,"ax",@progbits
.globl _start
.type _start, @function
_start:
    call main

1:
    ebreak
    j 1b

.size _start, .-_start