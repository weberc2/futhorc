package main

import (
	"context"
	"futhorc/pkg/futhorc"
	"log"
	"log/slog"
	"os"
	"runtime/pprof"
	"runtime/trace"
	"time"
)

func main() {
	if input := os.Getenv("LOG_LEVEL"); input != "" {
		var lvl slog.Level
		if err := lvl.UnmarshalText([]byte(input)); err != nil {
			log.Fatalf("parsing `LOG_LEVEL`: %v", err)
		}
		slog.SetLogLoggerLevel(lvl)
	}

	start := time.Now()
	defer func() { slog.Debug("completed", "elapsed", time.Since(start)) }()
	slog.Debug("started", "time", start)

	dir := "."
	if len(os.Args) > 1 {
		dir = os.Args[1]
	}

	pproff, err := os.Create("./run.pprof")
	if err != nil {
		log.Fatal(err)
	}
	defer pproff.Close()

	tracef, err := os.Create("./run.trace")
	if err != nil {
		log.Fatal(err)
	}
	defer tracef.Close()

	if err := pprof.StartCPUProfile(pproff); err != nil {
		log.Fatalf("starting pprof: %v", err)
	}
	defer pprof.StopCPUProfile()
	if err := trace.Start(tracef); err != nil {
		log.Fatalf("starting trace: %v", err)
	}
	defer trace.Stop()

	pipeline, err := futhorc.LoadPipeline(dir)
	if err != nil {
		log.Fatal(err)
	}

	if err := pipeline.Run(context.Background()); err != nil {
		log.Fatal(err)
	}
}
