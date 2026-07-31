// Returns number of characters copied, including the null terminator
int strcpy(char * restrict dst, const char * restrict src) {
    int i = 0;
    while ((dst[i] = src[i]) != '\0') {
        i++;
    }
    return ++i;
}

int main(void) {
    char* src = "Testing";
    char volatile* dst = (char volatile *) 0x2000'0000;

    return strcpy((char *) dst, src);
}