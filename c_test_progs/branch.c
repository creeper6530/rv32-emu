[[noreturn]] void _start() {
    volatile unsigned int *source = (volatile unsigned int *) 0x2000'00FF;
    volatile unsigned int *dest = (volatile unsigned int *) 0x2000'00AA;

    int cond = *source;
    if (cond % 2 == 0) {
        *dest = 123;
    } else {
        *dest = 456;
    }

    asm("ebreak");
    while (1) { };
}