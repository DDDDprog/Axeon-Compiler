#include <stdio.h>

void print_greeting(const char* name) {
    printf("Hello, %s! Welcome to the modern Axeon toolchain.\n", name);
    printf("This message was compiled by axeon, assembled by axeon-as, and linked by axeon-ld.\n");
}
