package main

func pump(ch chan int, done chan bool) {
	for value := 0; value < 3; value++ {
		ch <- value
	}

	done <- true
}

func main() {
	ch := make(chan int)
	done := make(chan bool)


	go pump(ch, done)

	for {
		select {
		case value := <-ch:
			println(value)
		case <-done:
			return
		}
	}
}
