#include <stdio.h>
#include <string.h>

#define GREETING "hello {world}"
#define LIMIT 8

static int table[LIMIT];

int fill(void) {
	for (int i = 0; i < LIMIT; i++) {
		table[i] = i * 2;
	}

	return table[LIMIT - 1];
}

int main(void) {
	printf("%s %d\n", GREETING, fill());

	return 0;
}
