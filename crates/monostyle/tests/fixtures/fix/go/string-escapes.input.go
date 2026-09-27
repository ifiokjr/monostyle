package main

func banner(name string) string {
	line := "line one\n"
	quoted := "she said \"hi\""


	return line + quoted + name
}

func main() {
	println(banner("go"))
}
