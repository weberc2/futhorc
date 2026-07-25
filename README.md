# Futhorc

> [!NOTE]
> This repository is a generated, read-only export of futhorc's source
> tree. Development happens elsewhere and changes arrive as snapshot
> commits; anything pushed directly here will be overwritten by the next
> export.

Futhorc is a static site generator for building my personal blog--**it's not
intended, recommended, or supported for broader uses**. It's written in Go
and uses a Communicating Sequential Processes (CSP) architecture for
efficient builds (see [the blog
post](https://blog.weberc2.com/posts/efficient-ssg-with-csp.html) for
details).

Tasks run through the repo-local runner: `./x build` compiles the CLI to
`bin/futhorc`, `./x test` runs the unit tests, and `./x render` renders a
site (`./x help` lists everything). Plain `go build ./cmd/futhorc` works
too.
