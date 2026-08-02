// Like `std::hint::black_box()` in Rust, prevents constant folding
#define BLACK_BOX(x) \
    ({ \
        typeof(x) _x = (x); \
        __asm__ volatile("" : "+r" (_x)); \
        _x; \
    })

// Returns number of characters copied, including the null terminator
static inline int strcpy(char * restrict dst, const char * restrict src) {
    int i = 0;
    while ((dst[i] = src[i]) != '\0') {
        i++;
    }
    return ++i;
}

int main(void) {
    char* src = "Testing";
    volatile char dst[8] = {0}; // Account for null terminator

    return strcpy((char *) dst, src);
}