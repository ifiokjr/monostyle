#include <stdio.h>

int main(void) {
	const char *message = "first part {kept} "
		"second part \"quoted\" "
		"third part";

	printf("%s\n", message);

	return 0;
}
