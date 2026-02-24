package example

import "strings"

// Greet returns a greeting string for the given name.
// If name is empty, it falls back to "World".
func Greet(name string) string {
	if name == "" {
		name = "World"
	}
	return "Hello, " + name + "!"
}

// Reverse returns the UTF-8 characters of s in reverse order.
func Reverse(s string) string {
	runes := []rune(s)
	for i, j := 0, len(runes)-1; i < j; i, j = i+1, j-1 {
		runes[i], runes[j] = runes[j], runes[i]
	}
	return string(runes)
}

// CountWords splits s on whitespace and returns the number of words.
func CountWords(s string) int {
	return len(strings.Fields(s))
}
