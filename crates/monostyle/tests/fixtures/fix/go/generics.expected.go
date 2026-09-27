package main

func Map[T any, U any](source []T, transform func(T) U) []U {
	result := make([]U, 0, len(source))

	for _, item := range source {
		result = append(result, transform(item))
	}

	return result
}

func main() {
	numbers := []int{1, 2, 3}

	println(len(Map(numbers, func(n int) string { return string(rune(n)) })))
}
