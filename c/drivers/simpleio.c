#include "simpleio.h"

inline void putchar(const unsigned char c) { *SIMPLEIO_OUTPUT = c; };

void puts(const char* restrict string) {
    *SIMPLEIO_FLAGS |= 1 << 1; // Lock output
    while (*string != '\0') {
        putchar(*string);
        string++;
    }
    *SIMPLEIO_FLAGS &= ~(1 << 1); // Unlock output
};

void flush(void) {
    *SIMPLEIO_FLAGS |= 1 << 2; // Flush output
};
