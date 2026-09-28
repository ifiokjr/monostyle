#include <stdio.h>

#define SWAP(a, b) do { \
	int temp = (a);      \
	(a) = (b);           \
	(b) = temp;          \
} while (0)

int main(void) {
	int x = 1;
	int y = 2;

	SWAP(x, y);
	printf("%d %d\n", x, y);

	return 0;
}
