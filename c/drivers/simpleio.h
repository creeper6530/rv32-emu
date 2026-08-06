#ifndef DRIVERS_SIMPLEIO_H
#define DRIVERS_SIMPLEIO_H

#include <stdarg.h>

#define SIMPLEIO_BASE 0xA000'0000
#define SIMPLEIO_INPUT_OFFSET 0x0
#define SIMPLEIO_OUTPUT_OFFSET 0x1
#define SIMPLEIO_FLAGS_OFFSET 0x2

#define SIMPLEIO_FLAGS_INPUT_LOCK (1 << 0)
#define SIMPLEIO_FLAGS_OUTPUT_LOCK (1 << 1)
#define SIMPLEIO_FLAGS_FLUSH_OUTPUT (1 << 2)

#define SIMPLEIO_INPUT ((volatile const unsigned char*)(SIMPLEIO_BASE + SIMPLEIO_INPUT_OFFSET))
#define SIMPLEIO_OUTPUT ((volatile unsigned char*)(SIMPLEIO_BASE + SIMPLEIO_OUTPUT_OFFSET))
#define SIMPLEIO_FLAGS ((volatile unsigned char*)(SIMPLEIO_BASE + SIMPLEIO_FLAGS_OFFSET))

void puts(const char* restrict string);
void putchar(const unsigned char c);
void flush(void);

// `gets()` has been removed from C11 for security reasons
// Returns the number of characters read (excluding the null terminator),
// or -1 on error
int fgets(char* restrict buffer, const unsigned int max_length);
unsigned char getchar(void);

void fmtuint(char* restrict buffer, unsigned int n, unsigned int base);
void fmtint(char* restrict buffer, signed int n, unsigned int base);

void vsprintf(char* restrict buffer, const char* restrict format, va_list args);
void sprintf(char* restrict buffer, const char* restrict format, ...);
void printf(const char* restrict format, ...);

int string_to_int(const char* restrict str, int* const restrict result);

#endif // DRIVERS_SIMPLEIO_H