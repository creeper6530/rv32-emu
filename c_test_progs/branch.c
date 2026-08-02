int main(void) {
    // Get the current stack pointer value
    volatile unsigned char *sp;
    asm volatile("mv %0, sp" : "=r"(sp));

    return *(sp - 1) % 2 == 0 ? 12 : 45;
}