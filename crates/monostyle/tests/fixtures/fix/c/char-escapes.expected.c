#include <stdio.h>

int main(void) {
	char letter = 'A';
	char control = '\0';
	char hex = '\x41';
	char octal = '\101';
	const char *path = "C:\\temp\\{kept}";

	printf("%c%c %d\n", letter, hex, control);

	return 0;
}
