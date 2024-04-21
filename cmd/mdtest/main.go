package main

import (
	"bytes"
	_ "embed"
	"fmt"
	"strings"

	"gitlab.com/golang-commonmark/markdown"
)

func main() {
	md := markdown.New()
	chunks := bytes.SplitAfterN(input, []byte("---\n"), 3)

	var sb strings.Builder
	md.Render(&sb, chunks[2])

	fmt.Println(sb.String())
}

//go:embed input.md
var input []byte
