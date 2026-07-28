[[noreturn]] static inline void ebreak(void) {
    asm volatile("ebreak");
}
