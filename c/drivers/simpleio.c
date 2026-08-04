#include "simpleio.h"

#define SETBIT(ptr, bit) *ptr |= bit;
#define CLEARBIT(ptr, bit) *ptr &= ~bit;

inline void putchar(const unsigned char c) { *SIMPLEIO_OUTPUT = c; };

void puts(const char* restrict string) {
    if (string == nullptr) {
        return;
    }

    SETBIT(SIMPLEIO_FLAGS, SIMPLEIO_FLAGS_OUTPUT_LOCK) // Lock output
    while (*string != '\0') {
        putchar(*string);
        string++;
    }
    CLEARBIT(SIMPLEIO_FLAGS, SIMPLEIO_FLAGS_OUTPUT_LOCK) // Unlock output
};

void flush(void) {
    SETBIT(SIMPLEIO_FLAGS, SIMPLEIO_FLAGS_FLUSH_OUTPUT) // Flush output
};

inline unsigned char getchar(void) { return *SIMPLEIO_INPUT; };

int fgets(char* restrict buffer, const unsigned int max_length) {
    if (max_length == 0) {
        return -1;
    }
    if (buffer == nullptr) {
        return -1;
    }

    SETBIT(SIMPLEIO_FLAGS, SIMPLEIO_FLAGS_INPUT_LOCK) // Lock input
    unsigned int i = 0;
    while (i < max_length - 1) {
        unsigned char c = getchar();
        if (c == '\n' || c == '\r') {
            break;
        }
        buffer[i] = c;
        i++;
    }
    buffer[i] = '\0'; // Null-terminate the string
    CLEARBIT(SIMPLEIO_FLAGS, SIMPLEIO_FLAGS_INPUT_LOCK) // Unlock input

    return i; // Return the number of characters read
};
