#define MEM_BASE ((volatile unsigned int *) (0x2000'0040))
#include "ebreak.h"

int main() {
    volatile unsigned int *ptr = MEM_BASE;
    *ptr = 0x101;
    ebreak();
}
