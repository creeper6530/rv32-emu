int main() {
    // `volatile` is used to prevent the compiler from optimizing away the write operation
    // (or putting it in a register). We can find the value in stack memory later.
    volatile int x = 0xC0FFEE;
    return 0;
}
