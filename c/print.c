#include "drivers/simpleio.h"

int main() {
    puts("Hello, world!\n");

    puts("Answer: ");
    flush();

    char buffer[100];
    int length = fgets(buffer, sizeof(buffer));
    if (length >= 0) {
        puts("You entered: ");
        puts(buffer);
        puts("\n");
    } else {
        puts("Error reading input.\n");
    }

    return 0;
}