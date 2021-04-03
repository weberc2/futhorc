---
Title: Installing Go on Linux & OS X
Date: 2016-01-19
---

This is a guide for Unix systems (OS X and Linux), but Windows users shouldn't
find it too difficult to figure out the equivalent commands for their platform.
I'm not assuming much prior knowledge, but readers should at least be
comfortable navigating around a Unix terminal, and any familiarity with
environment variables is helpful (a quick Google search for "environment
variables" should suffice). Without further ado:

<!-- more -->

1. [Download Go][1]
2. If you're on Linux, you'll need to pick an extraction location (the OS X
   installer should handle this part of the setup automatically). I usually
   install to `~/.go`, but it doesn't matter much. From your terminal, run:

    ``` bash
    tar -xvf ~/Downloads/go1.5.3.linux-amd64.tar.gz
    mv go ~/.go
    ```

    You'll also need to add `.go/bin` to your `$PATH` environment variable so you
    can run the go tool by its name (e.g., `go build` vs `~/.go/bin/go build`). Add
    this to the bottom of your `~/.bashrc` (this file sets up your terminal every
    time you log in):

    ``` bash
    # Add Go to your path
    PATH=$PATH:$HOME/.go/bin
    export $PATH
    ```

    To load those changes into your shell without having to log out and log
    back in, you can run: `source ~/.bashrc`. You should now be able to run
    `go version` and see something like: `go version go1.5.3 linux/amd64`.

3. Setup your `$GOPATH`. `$GOPATH` is a colon-delineated list of "workspace"
   directory paths. A workspace must have a `./src/` subdirectory. When the Go
   compiler encounters an import path in a source file, it will iterate over
   each workspace until it finds one that has the import path in its `src`
   subdirectory. I recommend keeping a single path in your `$GOPATH` for
   simplicity. Windows users will have to Google for "how to set environment
   variables" to follow along with this article. For OS X and Linux users, add
   the following to your `~/.bashrc` (Linux) or `~/.bash_profile` (OS X):

    ``` bash
    # Setup $GOPATH
    GOPATH=$HOME/Projects
    export GOPATH

    # When a go program is built, it will be added to $GOPATH/bin. In order to
    # run these programs by name, we need to add them to $PATH:
    PATH=$PATH:$GOPATH/bin
    export PATH
    ```

    Load those changes into your shell via `source ~/.bashrc` or `source
    ~/.bash_profile`, then create the directory: `mkdir -p ~/Projects/src`.

4. Test our setup with a "hello world" project. Make a directory for your
   project: `mkdir ~/Projects/src/hello`, then copy the following into
   `~/Projects/src/hello/main.go`:

    ``` go
    package main

    import "fmt"

    func main() {
    	fmt.Println("Hello, world!")
    }
    ```

    Install it via `go install hello`, and run it by invoking `hello`. If you
    don't see `Hello, world!`, then something bad happened.


[1]: https://golang.org/dl/
