#include <stdio.h>

int main(void) {
	const char *continued = "the text goes on \
and continues here";
	const char *escapes = "tab\t newline\n quote\" backslash\\";

	printf("%s %s\n", continued, escapes);

	return 0;
}
