package main

import "fmt"

const query = `
{
  "user": "ada",
  "roles": ["admin"]
}
`

func roles() []string {
	var names []string


	names = append(names, "admin")

	return names
}

func main() {
	fmt.Println(query, roles())
}
