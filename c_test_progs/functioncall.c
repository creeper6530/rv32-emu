// Like `std::hint::black_box()` in Rust, prevents constant folding
#define BLACK_BOX(x) \
    ({ \
        typeof(x) _x = (x); \
        __asm__ volatile("" : "+r" (_x)); \
        _x; \
    })

short mul_3(short x);

[[noreturn]] void _start() {
    short x = 5;
    short result = mul_3(BLACK_BOX(x));
    volatile unsigned short *ptr = (volatile unsigned short *) 0x2000'0321;
    *ptr = result;
    asm("ebreak");
    while (1) { };
}

__attribute__((noinline))
short mul_3(short x) {
    return x * 3;
}