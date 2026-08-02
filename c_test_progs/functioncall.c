// Like `std::hint::black_box()` in Rust, prevents constant folding
#define BLACK_BOX(x) \
    ({ \
        typeof(x) _x = (x); \
        __asm__ volatile("" : "+r" (_x)); \
        _x; \
    })

short mul_3(short x);

__attribute__((noinline))
short mul_3(short x) {
    return x * 3;
}

int main(void) {
    short x = 5;
    // The BLACK_BOX() macro prevents the compiler from optimizing away the function call to mul_3().
    return mul_3(BLACK_BOX(x));
}