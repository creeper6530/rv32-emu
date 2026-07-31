int main() {
    volatile unsigned int *ptr = (volatile unsigned int *) (0x2000'0040);
    *ptr = 0x101;

    return 0;
}
