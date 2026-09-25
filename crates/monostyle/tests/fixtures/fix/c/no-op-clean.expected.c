#include <stdio.h>

#define BRACED {"kept": 1}

int sum(int values[], int count) {
	int total = 0;

	for (int i = 0; i < count; i++) {
		total += values[i];
	}

	return total;
}

int main(void) {
	int values[] = {1, 2, 3};

	printf("%d\n", sum(values, 3));

	return 0;
}
