#define BLACK_BOX(x) \
    ({ \
        typeof(x) _x = (x); \
        __asm__ volatile("" : "+r" (_x)); \
        _x; \
    })

short variable = 100;

int strlen(const char *str) {
    int length = 0;
    while ((*str++) != '\0') {
        length++;
    }
    return length;
}

int main(void) {
    volatile short *ptr = &variable;

    const char *test_str = "Test String";
    for (int i = strlen(BLACK_BOX(test_str)); i > 0; i--) {
        *ptr += 1;
    }

    return variable; // Correct result should be 111 (100 + 11)
}