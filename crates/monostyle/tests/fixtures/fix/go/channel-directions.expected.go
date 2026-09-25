package main

func produce(out chan<- int, values []int) {
	for _, value := range values {
		out <- value
	}

	close(out)
}

func consume(in <-chan int, done chan<- bool) {
	for range in {
	}

	done <- true
}
