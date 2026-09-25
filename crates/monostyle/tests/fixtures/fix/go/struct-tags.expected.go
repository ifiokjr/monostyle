package main

type User struct {
	Name  string `json:"name,omitempty"`
	Age   int    `json:"age"`
	Notes string `json:"notes"`
}

func main() {
	user := User{Name: "ada", Age: 36}

	println(user.Name)
}
