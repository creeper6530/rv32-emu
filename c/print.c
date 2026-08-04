#include "drivers/simpleio.h"

int main() {
    puts("Hello, world!\n");

    puts("Enter string: ");
    flush();

    char buffer[100];
    fgets(buffer, sizeof(buffer));
    printf("String: %s\n", buffer);

    printf("Char: %c\n", 'z');
    printf("Signed int: %d\n", -12345);
    printf("Unsigned int: %u\n", 12345);
    printf("Hex: %x\n", 0xDEADBEEF);
    printf("Octal: %o\n", 07531);
    printf("Binary: %b\n", 0b101010);
    printf("Pointer: %p\n", buffer);

    return 0;
}