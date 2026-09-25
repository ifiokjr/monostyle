#include <stdio.h>

typedef int (*operation)(int, int);

static int add(int a, int b) { return a + b; }
static int mul(int a, int b) { return a * b; }

int apply(operation callback, int a, int b) {
	return callback(a, b);
}

int main(void) {
	operation table[] = {add, mul};


	printf("%d %d\n", apply(table[0], 2, 3), apply(table[1], 2, 3));

	return 0;
}
