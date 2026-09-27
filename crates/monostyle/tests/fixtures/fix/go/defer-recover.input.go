package main

import "fmt"

func safe() {
	defer func() {
		if r := recover(); r != nil {
			fmt.Println("recovered", r)
		}
	}()


	panic("boom")
}

func main() {
	safe()
}
