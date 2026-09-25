package main

type Point struct {
	X int
	Y int
}

type Shape struct {
	Name    string
	Points  []Point
	Comment string
}

func main() {
	shape := Shape{
		Name:    "triangle",
		Points:  []Point{{X: 0, Y: 0}, {X: 1, Y: 1}},
		Comment: `a "quoted" {note}`,
	}

	println(shape.Comment)
}
