#include "drivers/simpleio.h"

int fib(int n) { return (n < 2) ? n : fib(n - 1) + fib(n - 2); }

int main(void) {
    char buffer[100];
    int result;

    puts("Enter an integer: ");
    flush();
    fgets(buffer, sizeof(buffer));
    if (string_to_int(buffer, &result) != 0) {
        printf("Invalid input\n");
        return -1;
    }

    printf("Fibonacci of %d is: %d\n", result, fib(result));
    return 0;
}