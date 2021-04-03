---
Title: Go generics iterator sketch
Date: 2020-06-17
---

The new Go generics proposal and playground gives us something to play with.
Here's a sketch of what a basic iterator library could look like. It's based on
function types instead of interfaces; I think this gives better ergonomics than
interface-based designs, especially if the proposal drops its seemingly
arbitrary [restrictions for method types][0].

I'm not sure about returning a pointer to the type as opposed to a `(T, bool)`
tuple. In particular, I suspect this will cause unnecessary allocations, but I
haven't tested at all.

<!-- more -->

I'd love feedback:

* [HackerNews](https://news.ycombinator.com/item?id=23556737)
* [/r/golang](https://www.reddit.com/r/golang/comments/hb031l/go_generics_iterator_sketch/)
* [Twitter](https://twitter.com/weberc2)


[Playground](https://go2goplay.golang.org/p/WKouSq6mAh3)

```golang
package main

import (
	"fmt"
	"strings"
)

// I made `Iter` a function type instead of an interface because I wanted to
// hang methods off of it such as Iter.Map() and Iter.ForEach()--you can do
// this for functions but strangely not for interfaces--however, this doesn't
// work for methods like Iter.Map() because the proposal bizarrely prohibits
// types not defined on the receiver type. Specifically, we can't specify the
// output type for the map. So instead I made functions that take an iter and
// return an iter. Not a huge loss, especially since Go doesn't support chained
// methods (e.g., `iter.Map(...).Reduce(...)`) very well, esp wrt line
// wrapping).
type Iter(type T) func() *T

func next(type I)(iter Iter(I)) *I { return iter() }

func map_(type I, O)(iter Iter(I), f func(I) O) Iter(O) {
	return func() *O {
		if ptr := next(iter); ptr != nil {
			o := f(*ptr)
			return &o
		}
		return nil
	}
}

func forEach(type I)(iter Iter(I), f func(I)) {
	for ptr := next(iter); ptr != nil; ptr = next(iter) {
		f(*ptr)
	}
}

func SliceIter(type T)(slice []T) Iter(T) {
	return func() *T {
		if len(slice) < 1 {
			return nil
		}
		tmp := &slice[0]
		slice = slice[1:]
		return tmp
	}
}

func main() {
	forEach(
		map_(
			SliceIter([]string{"hello", "der"}),
			strings.ToUpper,
		),
		func(s string) { fmt.Println(s) },
	)
}

```

[0]: https://go.googlesource.com/proposal/+/refs/heads/master/design/go2draft-type-parameters.md#methods-may-not-take-additional-type-arguments
