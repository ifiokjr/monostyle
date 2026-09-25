package main

func lookup(table map[string]int, key string) int {
	if value, ok := table[key]; ok {
		return value
	}

	if _, ok := table["missing"]; !ok {
		return -1
	}

	return 0
}

func main() {
	println(lookup(map[string]int{"a": 1}, "a"))
}
