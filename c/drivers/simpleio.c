#include "simpleio.h"

#define SETBIT(ptr, bit) *ptr |= bit;
#define CLEARBIT(ptr, bit) *ptr &= ~bit;

// ------------------------------------------------------------------------------------------------

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

// ------------------------------------------------------------------------------------------------

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
    buffer[i] = '\0';                                   // Null-terminate the string
    CLEARBIT(SIMPLEIO_FLAGS, SIMPLEIO_FLAGS_INPUT_LOCK) // Unlock input

    return i; // Return the number of characters read
};

// ------------------------------------------------------------------------------------------------

void fmtuint(char* restrict buffer, unsigned int n, unsigned int base) {
    if (base < 2 || base > 36) {
        return; // Invalid base
    }

    // Find length first
    unsigned int temp = n;
    int length = 0;
    do {
        length++;
        temp /= base;
    } while (temp > 0);

    // Write digits from left to right
    for (int i = length - 1; i >= 0; i--) {
        buffer[i] = "0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ"[n % base];
        n /= base;
    }
    buffer[length] = '\0'; // Null-terminate the string
};

void fmtint(char* restrict buffer, signed int n, unsigned int base) {
    if (base < 2 || base > 36) {
        return; // Invalid base
    }

    if (n < 0) {
        *buffer++ = '-';
        n = -n;
    }

    unsigned int un = (unsigned int)n;

    fmtuint(buffer, un, base);
};

// ------------------------------------------------------------------------------------------------

void vsprintf(char* restrict buffer, const char* restrict format, va_list args) {
    while (*format != '\0') {
        if (*format == '%') {
            format++;
            switch (*format) {
            case 'c': { // Character
                // C standard promotes char to int
                int c = va_arg(args, int);

                *buffer++ = (char)c;
                break;
            }
            case 's': { // String
                const char* str = va_arg(args, const char*);
                while (*str != '\0') {
                    *buffer++ = *str++;
                }
                break;
            }
            case 'i':
            case 'd': { // Signed decimal integer
                signed int value = va_arg(args, signed int);
                char intbuf[12]; // Enough for 32-bit int
                char* intbuf_ptr = intbuf;

                fmtint(intbuf_ptr, value, 10);
                while (*intbuf_ptr != '\0') {
                    *buffer++ = *intbuf_ptr++;
                }
                break;
            }
            case 'u': { // Unsigned decimal integer
                unsigned int value = va_arg(args, unsigned int);
                char uintbuf[11]; // Enough for 32-bit unsigned int
                char* uintbuf_ptr = uintbuf;

                fmtuint(uintbuf_ptr, value, 10);
                while (*uintbuf_ptr != '\0') {
                    *buffer++ = *uintbuf_ptr++;
                }
                break;
            }
            case 'X':
            case 'x': { // Unsigned hexadecimal integer
                unsigned int value = va_arg(args, unsigned int);
                char uintbuf[9]; // Enough for 32-bit unsigned int in hex
                char* uintbuf_ptr = uintbuf;

                fmtuint(uintbuf_ptr, value, 16);
                while (*uintbuf_ptr != '\0') {
                    *buffer++ = *uintbuf_ptr++;
                }
                break;
            }
            case 'B':
            case 'b': { // Unsigned binary integer
                unsigned int value = va_arg(args, unsigned int);
                char uintbuf[33]; // Enough for 32-bit unsigned int in binary
                char* uintbuf_ptr = uintbuf;

                fmtuint(uintbuf_ptr, value, 2);
                while (*uintbuf_ptr != '\0') {
                    *buffer++ = *uintbuf_ptr++;
                }
                break;
            }
            case 'o': { // Unsigned octal integer
                unsigned int value = va_arg(args, unsigned int);
                char uintbuf[12]; // Enough for 32-bit unsigned int in octal
                char* uintbuf_ptr = uintbuf;

                fmtuint(uintbuf_ptr, value, 8);
                while (*uintbuf_ptr != '\0') {
                    *buffer++ = *uintbuf_ptr++;
                }
                break;
            }
            case 'p': { // Pointer (printed as hexadecimal)
                void* ptr = va_arg(args, void*);
                unsigned int value = (unsigned int)ptr;
                char uintbuf[11]; // Enough for 32-bit unsigned int in hex
                uintbuf[0] = '0';
                uintbuf[1] = 'x';
                char* uintbuf_ptr = uintbuf;

                // Start after "0x"
                fmtuint(uintbuf_ptr + 2, value, 16);
                while (*uintbuf_ptr != '\0') {
                    *buffer++ = *uintbuf_ptr++;
                }
                break;
            }
            case '%': { // Literal '%'
                *buffer++ = '%';
                break;
            }
            default:
                *buffer++ = '%';
                *buffer++ = *format;
                break;
            }
        } else {
            *buffer++ = *format;
        }
        format++;
    }

    *buffer = '\0'; // Null-terminate the string
}

void sprintf(char* restrict buffer, const char* restrict format, ...) {
    va_list args;
    va_start(args, format);

    vsprintf(buffer, format, args);

    va_end(args);
}

void printf(const char* restrict format, ...) {
    va_list args;
    va_start(args, format);

    char buffer[1024];
    vsprintf(buffer, format, args);

    va_end(args);
    puts(buffer);
};

// ------------------------------------------------------------------------------------------------

// https://cdecl.plus/?q=int%2A%20const%20restrict%20result
int string_to_int(const char* restrict str, int* const restrict result) {
    if (str == nullptr || result == nullptr) {
        return -1; // Invalid input
    }

    register int tmp = 0;
    int sign = 1;

    if (*str == '-') {
        sign = -1;
        str++;
    } else if (*str == '+') {
        str++;
    }

    while (*str != '\0') {
        if (*str < '0' || *str > '9') {
            return -1; // Invalid character
        }
        tmp = (tmp * 10) + (*str - '0');
        str++;
    }
    tmp *= sign;

    *result = tmp;
    return 0; // Success
}
