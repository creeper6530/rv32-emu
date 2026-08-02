// Like `std::hint::black_box()` in Rust, prevents constant folding
#define BLACK_BOX(x) \
    ({ \
        typeof(x) _x = (x); \
        __asm__ volatile("" : "+r" (_x)); \
        _x; \
    })

short data = 100;
short bss;

int strlen(const char *str) {
    int length = 0;
    while ((*str++) != '\0') {
        length++;
    }
    return length;
}

int main(void) {
    volatile short *bss_ptr = &bss;
    volatile short *data_ptr = &data;

    const char *test_str = "Test String"; // rodata
    for (int i = strlen(BLACK_BOX(test_str)); i > 0; i--) {
        *bss_ptr += 1;
    }

    return *bss_ptr + *data_ptr; // Correct result should be 111 (11 + 100)
}