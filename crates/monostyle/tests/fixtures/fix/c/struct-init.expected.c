#include <stdio.h>

typedef struct {
	const char *name;
	int length;
	int flags;
} Entry;

int main(void) {
	Entry entries[] = {
		{.name = "first", .length = 5, .flags = 0x1},
		{.name = "second {brace}", .length = 14, .flags = 0x2},
	};

	printf("%s %d\n", entries[1].name, entries[1].flags);

	return 0;
}
