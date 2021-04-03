---
Title: Getting started with Go, 2018 edition
Date: 2018-10-14
---

A little over 2.5 years ago, I wrote a tutorial about [installing Go][0]. Since
then, one of the more significant changes to the Go ecosystem has been the
addition of [modules][1], which effectively does away with the hardest part of
installing Go--`$GOPATH`. This change occurred in the latest version: Go 1.11.

In addition to installing Go, I wanted to make a guide that can get you from
nothing to a real project in half an hour. Most languages focus their
introductory material on the language and briefly cover setting up a toy
program. When you're done, you realize you have no idea how to build a
multi-file program, how to add dependencies (or at least how to add them in a
way that won't break other things on your system), how to get an editor up
and running, etc.

I'm not going to focus much at all on Go the language here, since it's super
easy to learn and there are already many great tutorials (the official
[Tour][2] is probably not a bad place to start). I'm only going to go deep
enough to give you a lay of the land; if I've done my job, it should be easy
enough to Google for specific resources on any given topic (for example,
testing).

Now without further ado...

<!-- more -->

<!--
NOTE: headers start at h3 b/c h1 is for site header and h2 is for post title
-->

# Installing Go

Here are the updated installation instructions:

1. [Download Go][3]
2. Install it

    2.1. If you're on Linux, untar it and put it somewhere like
         `/usr/local/go`.

    2.2. If you're on OSX, run the installer. This will install go to
         `/usr/local/go`.
3. Add the `go` binary to your `$PATH`. This is probably just editing the line
   in your  `~/.bashrc` or `~/.bash_profile` or etc that looks like this:
   `PATH=$PATH:...` by suffixing it with `:/usr/local/go/bin` (or the `./bin`
   directory inside of the Go installation directory). Now if you run
   `go version`, you should get `go version go1.11.1 linux/amd64` or
   comparable.

That's it. Now you can compile any Go program.

# Text Editor

Go has the highest quality text editor plugins of any programming language I've
used. The plugins all have the features you would expect from an IDE--the
ability to get the type information for a symbol or to get its documentation or
to navigate to its source code. I'm aware of high-quality plugins for VS Code,
vim, emacs, and Sublime, but there are probably others.

Importantly, all text editor plugins *should* support running `gofmt` on save
(most do this by default). This program formats your code with the same style
that is used across virtually all Go programs. No more bickering about style
in code reviews.

Given that Go 1.11 landed in the last month or two, many tools are still adding
support for Go modules, so you will likely see some bugs in some tools for the
next month or so. I recommend VS Code plus the Go plugin--in my opinion,
it's the easiest, most stable way to work with Go, especially if you're more
comfortable in a GUI environment (although this vim user has found himself
using VS Code more and more lately).

# Hello world

Create a new project directory anywhere on your system, say `/tmp/hello`. Now
copy and paste the following into `/tmp/hello/main.go`:

```Go
package main

import "fmt"

func main() { fmt.Println("Hello, world") }
```
This imports the `fmt` package from the standard library and uses it to print
`Hello, world`. Now, in your `hello` directory, run `go run main.go` to run
the file, or `go build` to build a `./hello` binary that you can run.

Note that the package is called `main` but lives in a directory called
`hello`, and the result of `go build` is a binary named `hello`. In Go, a
directory constitutes a *package*, and all `.go` files in the package must
have the same package declaration at the top. Running `go build` in a `main`
package directory will spit out an executable with the same name as the
directory that it lives in (modulo the `.exe` extension on Windows).

Note also that there were no project configuration files, so you didn't need
to learn a new set of configuration options or configuration file syntax. You
didn't need to figure out how to point the compiler at your source files or
tell the compiler the order in which it needs to process them. `go build` is
sufficient to build most Go programs.

# Dependencies

Dependency management is a critical function for writing software, yet most
languages' "getting started" guides don't guide you to the best practices if
they even broach the topic. Figuring out the right dependency management tool
and the right way to use it was left as an exercise for the reader, but frankly
it's a really complex exercise, especially since many ecosystems have multiple
dependency management systems that don't always play nicely together and each
has their respective tradeoffs.

As previously mentioned, Go recently standardized around modules, and while I
was skeptical initially, it seems to have done a pretty good job at solving the
problem (maybe I'll change my mind over time).

In your project directory, run `go mod init <module-name>` (note that your
project directory can contain many package directories or it can be a
single-package project in which the project directory and the package
directory are the same). This will create a `go.mod` file which contains
information about your project's dependencies. For the most part, the `go`
tool will manage this file (and its sibling, `go.sum`) for you; the file is
human-readable and easy to understand, but you should only need to touch it
very rarely. To add dependencies, all you need to do is import them in your
source code and run `go build`. The Go tool will pick a version of your
dependencies, update your `go.mod` and `go.sum` files, download the
dependencies to your system, and build the binary. Let's try it:

1. In your `/tmp/hello` directory, run `go mod init hello`.
2. Modify your `main.go` by replacing it with the following:
    ```Go
    package main

    import (
        "github.com/fatih/color"
    )

    func main() {
        color.Cyan("Hello, world!")
    }
    ```
3. Run `go build`, and observe it downloading your dependency (and its
   transitive dependencies):
    ```
    go: finding github.com/fatih/color v1.7.0
    go: downloading github.com/fatih/color v1.7.0
    go: finding github.com/mattn/go-isatty v0.0.4
    go: finding github.com/mattn/go-colorable v0.0.9
    go: downloading github.com/mattn/go-isatty v0.0.4
    go: downloading github.com/mattn/go-colorable v0.0.9
    ```
    You'll only see this output once since those dependencies get cached.
    Regardless of whether or not you've run `go build` before, you should see a
    `hello` binary, just as before. Running it should produce the same output,
    but in cyan.

    Note that Go pulls dependencies directly from version control, and it uses
    Git, Subversion, and Mercurial to do this, so make sure you have those
    installed on your system.

# Publishing packages

Go doesn't have a package repository; to "publish" packages, you just push your
code up to Github or BitBucket or wherever (you can even run your own
git/hg/svn server). No need to write CI scripts to publish packages for you.

# Testing

Go has unit tests built in, so you don't need to worry about figuring out what
unit test library or test runner to install nor how to run them. Just use the
standard library `testing` package and run your tests with `go test`. Test
files can live wherever you want them to, but its relatively common to put
them alongside the source code. Test files are suffixed by `_test.go`, and are
treated specially by the Go toolchain. Within a test file, functions that start
with `Test` and take a single `*testing.T` argument (no returns) are executed
as tests. That argument has methods attached to fail, skip, etc.

Here's a quick toy example:

```go
package arithmetic

import "testing"

func TestAdd(t *testing.T) {
    if result := Add(1, 2); result != 3 {
        t.Errorf("Wanted 3, got %d", result)
    }
}
```

# Documentation

Go doesn't require you to learn a special documentation syntax like javadoc or
Sphinx; it just pulls documentation from your normal code comments. If you push
your repo to Github or Bitbucket or similar, you can see its documentation
automatically via (for example) https://godoc.org/github.com/fatih/color. No
need to configure a CI job to build or publish documentation packages. There
is also a subcommand on the `go` tool called `go doc` which takes a symbol
identifier (such as `fmt.Printf`) and returns the documentation associated with
that symbol; check out `go help doc` for more details.

# Other tools

Go also has support for the following:

* Benchmarks (via `go test`)
* CPU/Memory profiling (via `go tool pprof`)
* Linting (via `golint` and third party linters)
* Code coverage (via `go tool cover`)
* Debugging (via [`delve`][4])

For a truly comprehensive list, check out [Awesome Go][5]

# Contact

For questions, corrections, suggestions, or criticism, hit me up via [email][6]
or [Twitter][7].

# Edit (2018-10-20)

Updated according to some feedback.

* Added an example to the test section per [/u/sc3nner][8] on
  [the Reddit thread for this post][9].
* Removed the reference to `godoc` from the "Documentation" section per
  [/u/qu33ksilver][10] on [Reddit][9].

[0]: ./installing-go-on-linux-and-osx.md
[1]: https://github.com/golang/go/wiki/Modules
[2]: https://tour.golang.org
[3]: https://golang.org/dl/
[4]: https://github.com/derekparker/delve
[5]: https://github.com/avelino/awesome-go
[6]: mailto:weberc2@gmail.com
[7]: https://twitter.com/weberc2
[8]: https://www.reddit.com/user/sc3nner
[9]: https://www.reddit.com/r/golang/comments/9odqor/getting_started_with_go_2018_edition/
[10]: https://www.reddit.com/user/qu33ksilver
