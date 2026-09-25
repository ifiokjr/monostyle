#include <stdio.h>

int main(void) {
	double ratio = 3.0 / 4.0;
	size_t total = (size_t)(ratio * 100.0);
	int clipped = total > 90 ? 90 : (int)total;

	clipped += 1;

	printf("%zu %d\n", total, clipped);

	return 0;
}
