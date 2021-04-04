---
Title: Go's interfaces and nil by example
Date: 2016-10-30
Tags: [golang]
---

I've recently been involved in conversations with a few Go developers who have
expressed frustration about Go's interfaces with respect to nil. It seems not
everyone understands that interfaces are a reference type (like a pointer), and
they can reference other reference types (i.e., pointers, maps, slices, etc).
Because all reference types have a nil (zero) value, an interface can be nil
or an interface can reference a nil pointer; people may fail to realize that
the nility of the interface is independent from the nility of the thing it
points to, and when we ask (for example) `if err != nil`, we're actually asking
*is the interface nil?*, not *is the value behind the interface nil?*. Here are
a few examples that will (*hopefully*) demonstrate this clearly:

<!-- more -->

```go
var nilInterface interface{} = nil
println("nil interface is nil?:", nilInterface == nil) // true
println(nilInterface) // (0x0, 0x0)
```

Note that printing the nil interface gives `(0x0, 0x0)`. An interface has 2
components: type information and value information. In the case of a nil
interface, both components are set to zero.

```go
var value int = 10
var interfaceToValue interface{} = value
println("interface-to-value is nil?:", interfaceToValue == nil) // false
println("value address:", &value)
println(interfaceToValue)
```

Here, we see the address of the value is something like `0xc420041f08`, and
printing the interface gives: `(0x52bc0,0xc420041f08)`. The value component is
the address of the value from which we created the interface (this is what we
mean when we say interfaces are reference types). The type component is also
non-zero.

```go
var p *int = nil
var interfaceToNilPtr interface{} = p
println("interface-to-nil-ptr is nil?:", interfaceToNilPtr == nil) // false
println("nil ptr:", p)
println(interfaceToNilPtr)
```

When we test an interface to a nil `*int` for nility, we see that it is not
nil! This is probably the more surprising case for some developers. When we
print the nil pointer, we see that it's address is `0x0`, the nil address. We
also see that printing the interface gives `(0x52bc0,0x0)`. Here we have type
information, but our value pointer is nil.

```go
var i int = 10
var p2 *int = &i
var ifaceToNonNilPtr interface{} = p2
println("iface-to-non-nil-ptr is nil?:", ifaceToNonNilPtr == nil) // false
println("non-nil-pointer:", p2)
println(interfaceToNonNilPtr)
```

Finally, we have an interface to a non-nil `*int`. When we print the non-nil
pointer, we see its address is something like `0xc420041f10`, and printing the
interface gives `(0x52bc0,0xc420041f10)`. Notice that the type information
`(0x52bc0)` is the same as the previous nil-`*int` example (because a nil
`*int` has the same *type* but different *value* than a non-nil `*int`). Also
notice that again, the pointer's value matches the value in the interface.

The complete, runnable example can be found [here][1].

[1]: https://play.golang.org/p/3uCXcaURaS
