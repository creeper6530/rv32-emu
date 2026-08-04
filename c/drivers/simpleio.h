#ifndef DRIVERS_SIMPLEIO_H
#define DRIVERS_SIMPLEIO_H

#define SIMPLEIO_BASE 0xA000'0000
#define SIMPLEIO_INPUT_OFFSET 0x00
#define SIMPLEIO_OUTPUT_OFFSET 0x04
#define SIMPLEIO_FLAGS_OFFSET 0x08

#define SIMPLEIO_INPUT                                                         \
    ((const unsigned char*)(SIMPLEIO_BASE + SIMPLEIO_INPUT_OFFSET))
#define SIMPLEIO_OUTPUT                                                        \
    ((unsigned char*)(SIMPLEIO_BASE + SIMPLEIO_OUTPUT_OFFSET))
#define SIMPLEIO_FLAGS ((unsigned char*)(SIMPLEIO_BASE + SIMPLEIO_FLAGS_OFFSET))

void puts(const char* restrict string);
void putchar(const unsigned char c);
void flush(void);

#endif // DRIVERS_SIMPLEIO_H