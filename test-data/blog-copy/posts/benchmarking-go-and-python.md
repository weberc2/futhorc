---
Title: Benchmarking Go and Python
Date: 2016-02-06
---

Sometimes I'm curious about the performance of different languages. At work, I
usually write Python, but I often find tasks that are inherently parallelizable
and could thus benefit from parallel execution. Unfortunately, Python is
notoriously difficult to parallelize[^a]. In one case, we needed to validate
that a table of values of a particular type could be convertible into a values
of a different type based on some known set of conversion rules. Since Go is a
great language for writing concurrent programs (and executing them in
parallel), I decided to compare a sequential Python implementation to
sequential and parallel Go implementations.

<!-- more -->

To simplify the problem for benchmarking, I decided to constrain the input type
to strings and the output type to integers, and the "conversion rule" is
`strconv.Atoi()` in Go and `int(string)` in Python. In all examples, I'm
reading comma-separated string values from stdin, building a 2D string array in
memory, and then timing the conversion of strings to ints. I didn't want to
include reading from stdin in my timing, because that would make my benchmarks
dependent upon the performance of the data source (a network, the file system,
etc). Reading from memory is much less variable.

In Python, the code looks like this:

``` py
def validate_rows(rows, col_size):
    for row_id, row in enumerate(rows):
        if len(row) != col_size:
            msg = "Row {} has {} cells, but expected {}\n"
            print(msg.format(row_id, len(row), col_size))
            continue
        for col_id, cell in enumerate(row):
            try:
                int(cell)
            except ValueError as e:
                print("Err at ({}, {}): {}".format(col_id, row_id, e))
```

And in Go:

``` go
func validateRows(rows [][]string, colSize int) {
    for rowID, row := range rows {
        if len(row) != colSize {
            msg := "Row %d has %d cells, but expected %d\n"
            fmt.Fprintf(os.Stderr, msg, rowID, len(row), colSize)
            continue
        }
        for colID, cell := range row {
            if _, err := strconv.Atoi(cell); err != nil {
                msg := "Err at (%d, %d): %v\n"
                fmt.Fprintf(os.Stderr, msg, colID, rowID, err)
            }
        }
    }
}
```

I also created a simple program that deterministically generates pseudo-random
integer data in 2D CSV format and writes it to stdout:

``` bash
$ go run csvgen.go
USAGE: csvgen <col-count> <row-count>
```

This lets me pipe data into the Go and Python versions, like so[^b]:

``` bash
# Sequential Python; 10,000 rows, 1,000 columns
$ go run csvgen.go 1000 10000 | python3 sequential.py
Beginning validation...
Validated 10000 rows of 1000 cells in 0:00:02.990360

# Sequential Go; 10,000 rows, 1,000 columns
$ go run csvgen.go 1000 10000 | go run sequential.go
Beginning validation...
Validated 10000 rows of 1000 cells in 664.804988ms
```

The Go program was about 4.5 times faster than the Python program. Running this
example over and over again on my MacBook Pro produces consistent results, with
the Go program outperforming the Python variant by 4-6 times.

But what about parallelism? I'm not at all familiar with parallelism in Python,
so I didn't attempt it (perhaps a reader could supply an implementation, and
contact me on [Twitter][1]?), but Go makes parallelism very easy, so I decided
to give it a shot:

``` go
func validateParallel(rows [][]string, coreCount int) {
    wg := sync.WaitGroup{}
    wg.Add(coreCount) // Add `coreCount` goroutines to the WaitGroup
    
    // divide `rows` into `coreCount` blocks of rows, and then dispatch a
    // goroutine to process each block.
    for _, block := range subslice(rows, coreCount) {
        // Create a new variable exclusively for the goroutine that corresponds
        // to this loop iteration. All goroutines can't share one variable,
        // because the variable will be pointing to the last block returned by
        // subslice() before the first goroutine is kicked off, meaning all
        // goroutines would be operating on the last block and the previous
        // blocks would be ignored.
        block := block
        
        go func() {
            validateRows(block, len(rows[0]))
            wg.Done() // signal that this goroutine has finished execution
        }()
    }
    
    wg.Wait() // block until `wg.Done()` has been called `coreCount` times
}
```

This version takes a 2D input string array and breaks it into `coreCount`
subarrays, where `coreCount` is intended to be the number of cores on the
machine (although it can be any number between 1 and `len(rows)`). It then
dispatches one goroutine (a lightweight, cooperative thread) per subarray,
which invokes the `validateRows()` function from the sequential Go code snippet
above. It also waits for each goroutine to finish before returning. Here's how
the function is called (without timing or print statements):

``` go
    // query the machine's CPU core count
    coreCount := runtime.NumCPU()
    
    // allow the Go runtime to spin up (at most) `coreCount` threads
    runtime.GOMAXPROCS(coreCount)
    
    // pass the input rows and coreCount to validateParallel()
    validateParallel(rows, coreCount)
```

This version does about twice as well as the sequential Go algorithm for all
inputs I tested:

``` bash
$ go run csvgen.go 1000 10000 | go run parallel.go
GOMAXPROCS: 4
Beginning validation...
Validated 10000 rows of 1000 cells in 299.069099ms
```

The performance ratio holds even if we bump up the input volume by an order of
magnitude:

``` bash
# Sequential Python; 100,000 rows, 1,000 columns
$ csvgen 1000 100000 | python3 sequential.py
Beginning validation...
Validated 100000 rows of 1000 cells in 0:00:30.148827

# Sequential Go; 100,000 rows, 1,000 columns
$ csvgen 1000 100000 | go run sequential.go
Beginning validation...
Validated 100000 rows of 1000 cells in 6.700995661s

# Parallel Go; 100,000 rows, 1,000 columns
$ csvgen 1000 100000 | go run parallel.go
GOMAXPROCS: 4
Beginning validation...
Validated 100000 rows of 1000 cells in 2.985182403s
```

Besides performance, one of the nice things about the Go implementations is
that they're not much less readable than the Python implementation. In fact,
I've found that the prevalence of types and the absence of features (no
inheritance, decorators, exceptions, etc) make Go very quick to learn and easy
to reason about.

Feel free to try it yourself, or perhaps improve on my implementations. The
source code for this case study can be found [here][2]. Share your results (or
any other thoughts you had on this post) with me on [Twitter][1].

[^a]: I got a lot of flack for this from a lot of Python folks who insisted
    that parallelism in Python was easy, but the fastest parallel Python
    implementation provided to me was still *twice as slow* as the *sequential
    Python implementation*. I've updated my benchmark repo to include the 3
    parallel Python implementations.

[^b]: Several folks were confused about whether or not the compilation of the
    Go programs (which take place during `go run`) were included in the
    benchmarks. They are not; the Go and Python programs are all responsible
    for timing themselves. This is because I wanted to be able to start time
    *after* each program had loaded its input data into a 2D-array in memory,
    so the I/O wasn't included in the benchmarks. 

[1]: https://twitter.com/weberc2
[2]: https://bitbucket.org/weberc2/csv-validation-benchmarks
