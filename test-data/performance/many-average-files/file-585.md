---
Title: Deploying Go apps on Docker scratch images
Date: 2018-11-04
---

NOTE: If you're here for the TL;DR, skip to the bottom.

A few months ago I built out some monitoring infrastructure at work using Go. I
deployed it to ECS (a Docker orchestrator, functionally similar to Kubernetes),
and for fun I decided to see how minimal I could make the image. I've used
Alpine base images before (which weigh in at about 5 MB and usually another 5 MB
for a small Go binary), but being that Go advertises itself as requiring only a
Linux kernel (most programming languages depend on an interpreter, a VM, and/or
system libraries--the latter abstract over kernel functionality and sometimes
provide a stable interface to the kernel), I wanted to see how true or practical
this was, and I wanted to better understand the things that I was taking for
granted by using distros.

As a matter of context, Docker has a special base image called `scratch` which
is empty--an application running on a scratch base image only has access to the
kernel (at least to the extent that containers provide isolation).

<!-- more -->

NOTE: This approach will not work if your Go application needs to subprocess out
to other programs (e.g., git) or if it uses pretty much any library that uses
CGo (these almost always depend on libc if not other libaries).

NOTE: The build steps may vary if you're not using Go modules.

Typically, to build a Docker image for a Go application, you write a Dockerfile
that specifies a base Docker image with the Go toolchain installed, copies in
the source code from the host machine, and invokes the compiler against it to
produce the binary before committing the changes to the final image. It looks
something like this:

```Dockerfile
FROM golang

WORKDIR /workspace

# Assuming the source code is collocated to this Dockerfile, copy the whole
# directory into the container that is building the Docker image.
COPY . .

RUN go build -o /myapp

# When a container is run from this image, run the binary
CMD /myapp
```

The toolchain and its dependencies (git, mercurial, etc) weigh a few hundred MBs
(never mind the weight of the distribution itself), so it's common to use a
Docker feature called [multi-stage Builds][1] to copy the binary artifact from
this first image into a second image without the toolchain. So we now have
something like this:

```Dockerfile
FROM golang

WORKDIR /workspace

COPY . .

RUN go build -o /myapp

FROM scratch

# Copy the artifact from the first build stage into the second stage (which will
# become the final image)
COPY --from=0 /myapp /myapp
```

Note that we've removed the `CMD /myapp` line because (probably for good reasons
that I don't fully understand), that actually runs `/bin/sh -c /myapp`, so
running the Docker image with the default command would otherwise give the
error: `docker: Error response from daemon: OCI runtime create failed:
container_linux.go:348: starting container process caused "exec: \"/bin/sh\":
stat /bin/sh: no such file or directory": unknown` (which is particularly
unhelpful). At least now, running the image without specifying a command will
give a more helpful error: `docker: Error response from daemon: No command
specified.`

This will work for something like hello world, but real apps have a few more
requirements. First of all, some packages in the standard library will try to
link against system libraries by default (IIRC, `net` specifically prefers to
use the system DNS resolver where possible). This means that containers created
from this image will fail at runtime because the binary can't find the system
libraries (because they don't exist). However, pure-Go implementations will be
used if the compiler is invoked with `CGO_ENABLED=0`:

```Dockerfile
FROM golang

WORKDIR /workspace

COPY . .

RUN CGO_ENABLED=0 go build -o /myapp

FROM scratch

COPY --from=0 /myapp /myapp
```

Now we can get past the dynamic linking error, but we run into another curiosity
if our application needs to make HTTPS requests (or pretty much anything else
that needs to use SSL): `Get https://www.google.com: x509: certificate signed by
unknown authority`. This error comes from `net/http`--not from Docker. We can
avoid this error by not handling it in the program: `rsp, _ := http.Get(url)`.
Kidding. This error is telling us that the HTTP library can't find the certs
required to establish an SSL connection (or something--I really understand SSL
much less than I probably should). Basically scratch also doesn't come with
these certs, so how do we get them into our final image? Copy them from the
builder stage!

```Dockerfile
FROM golang

WORKDIR /workspace

COPY . .

RUN CGO_ENABLED=0 go build -o /myapp

FROM scratch

COPY --from=0 /myapp /myapp

COPY --from=0 /etc/ssl/certs/ca-certificates.crt /etc/ssl/certs/
```

Now the application should run fine; however, by default, Docker images are run
as root user--to follow security best practices, we should use a nonroot user.
However, `scratch` doesn't have programs like `adduser` nor even `echo`, so we
can't `echo "$USER_INFO" >> /etc/passwd`, but we _can_ create a file in the
builder stage and copy _that_ over to the final stage:

```Dockerfile
# This is the first stage, for building things that will be required by the
# final stage (notably the binary)
FROM golang

# Assuming the source code is collocated to this Dockerfile
COPY . .

# Build the Go app with CGO_ENABLED=0 so we use the pure-Go implementations for
# things like DNS resolution (so we don't build a binary that depends on system
# libraries)
RUN CGO_ENABLED=0 go build -o /myapp

# Create a "nobody" non-root user for the next image by crafting an /etc/passwd
# file that the next image can copy in. This is necessary since the next image
# is based on scratch, which doesn't have adduser, cat, echo, or even sh.
RUN echo "nobody:x:65534:65534:Nobody:/:" > /etc_passwd

# The second and final stage
FROM scratch

# Copy the binary from the builder stage
COPY --from=0 /myapp /myapp

# Copy the certs from the builder stage
COPY --from=0 /etc/ssl/certs/ca-certificates.crt /etc/ssl/certs/

# Copy the /etc_passwd file we created in the builder stage into /etc/passwd in
# the target stage. This creates a new non-root user as a security best
# practice.
COPY --from=0 /etc_passwd /etc/passwd

# Run as the new non-root by default
USER nobody
```

It turns out this is all we _need_ to do. The certs and /etc/passwd file are
negligible in terms of size, so the final image is dominated by the size of your
executable. My small executable weighed 4.5 MB uncompressed and ECR (Amazon's
Docker Hub analog) reported it as 2.5 MB (presumably this is compressed). There
are a number of tricks for reducing your binary size further, but I'll leave
those as an exercise for the reader.

There is one last issue with this Dockerfile: rebuilds will have to pull the
dependencies every time, which takes a while (especially tedious if you're
iterating on the Dockerfile itself). To ameliorate this, we'll add _just_ the
`go.mod` and `go.sum` dependencies and run `go mod download` _before_ we copy in
the rest of the source code--in so doing, the Docker build cache will only
redownload dependencies when either the `go.mod` or `go.sum` files have changed:

```Dockerfile
# This is the first stage, for building things that will be required by the
# final stage (notably the binary)
FROM golang

# Copy in just the go.mod and go.sum files, and download the dependencies. By
# doing this before copying in the other dependencies, the Docker build cache
# can skip these steps so long as neither of these two files change.
COPY go.mod go.sum ./
RUN go mod download

# Assuming the source code is collocated to this Dockerfile
COPY . .

# Build the Go app with CGO_ENABLED=0 so we use the pure-Go implementations for
# things like DNS resolution (so we don't build a binary that depends on system
# libraries)
RUN CGO_ENABLED=0 go build -o /myapp

# ...

```

This wasn't much more work than building off of an Alpine base image, and I
deployed it this way. The size advantage over Alpine is still only ~5 MB, and
while that's a significant percentage, the impact on your workflow or
deployments will hardly be perceptible (largely due to caching). The biggest
reason to choose scratch over alpine is security--reduced attack surface and all
that--[which may be a more practical concern than I originally thought][0].

<!--
NOTE: headers start at h3 b/c h1 is for site header and h2 is for post title
-->

# Contact

Please share corrections, comments, or feedback on [Reddit][2] or [Twitter][3].

# TL;DR

By using a `scratch` base image, we save about ~5MB over Alpine base images and
we ship with a smaller attack surface.

```Dockerfile
# This is the first stage, for building things that will be required by the
# final stage (notably the binary)
FROM golang

# Copy in just the go.mod and go.sum files, and download the dependencies. By
# doing this before copying in the other dependencies, the Docker build cache
# can skip these steps so long as neither of these two files change.
COPY go.mod go.sum ./
RUN go mod download

# Assuming the source code is collocated to this Dockerfile
COPY . .

# Build the Go app with CGO_ENABLED=0 so we use the pure-Go implementations for
# things like DNS resolution (so we don't build a binary that depends on system
# libraries)
RUN CGO_ENABLED=0 go build -o /myapp

# Create a "nobody" non-root user for the next image by crafting an /etc/passwd
# file that the next image can copy in. This is necessary since the next image
# is based on scratch, which doesn't have adduser, cat, echo, or even sh.
RUN echo "nobody:x:65534:65534:Nobody:/:" > /etc_passwd

# The second and final stage
FROM scratch

# Copy the binary from the builder stage
COPY --from=0 /myapp /myapp

# Copy the certs from the builder stage
COPY --from=0 /etc/ssl/certs/ca-certificates.crt /etc/ssl/certs/

# Copy the /etc_passwd file we created in the builder stage into /etc/passwd in
# the target stage. This creates a new non-root user as a security best
# practice.
COPY --from=0 /etc_passwd /etc/passwd

# Run as the new non-root by default
USER nobody
```

# EDIT

* Thanks to Reddit user [ROL_A][4] for correcting a typo in the COPY statement
  that copied the /etc/passwd into the target stage, as well as for pointing out
  that Go1.11 modules are disabled when building within GOPATH; this was fixed
  by changing the `WORKDIR`.

[0]: https://www.securityweek.com/code-execution-alpine-linux-impacts-containers
[1]: https://medium.com/travis-on-docker/multi-stage-docker-builds-for-creating-tiny-go-images-e0e1867efe5a
[2]: https://www.reddit.com/r/golang/comments/9u7qnl/deploying_go_apps_on_docker_scratch_images/
[3]: https://twitter.com/weberc2
[4]: https://www.reddit.com/user/ROL_A
