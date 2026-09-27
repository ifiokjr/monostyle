package main

func scan(needle int, values []int) string {
	for i, value := range values {
		if value == needle {
			goto found
		}

		_ = i
	}


	return "missing"

found:
	return "found"
}
