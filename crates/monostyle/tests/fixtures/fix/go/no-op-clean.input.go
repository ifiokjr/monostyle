package main

import "fmt"

func tally(scores []int) int {
	total := 0

	for _, score := range scores {
		total += score
	}

	return total
}

func main() {
	fmt.Println(tally([]int{1, 2, 3}))
}
