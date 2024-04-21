package actor

import (
	"context"
	"errors"
	"fmt"
	"log/slog"
)

type BaseCallback func(context.Context) error

type Base struct {
	Name        string
	Concurrency int
	Callback    BaseCallback
}

func (actor *Base) Run(ctx context.Context) error {
	slog.Debug("starting actor", "name", actor.Name)
	defer slog.Debug("closing actor", "name", actor.Name)
	if actor.Concurrency > 1 {
		results := make(chan error)
		for i := range actor.Concurrency {
			go func(i int) {
				results <- actor.runIndividually(ctx)
			}(i)
		}
		for range actor.Concurrency {
			if err := <-results; err != nil {
				return err
			}
		}
		return nil
	}
	return actor.runIndividually(ctx)
}

func (actor *Base) runIndividually(ctx context.Context) error {
	for j := 0; true; j++ {
		if err := actor.Callback(ctx); err != nil {
			if errors.Is(err, ErrStop) {
				return nil
			}
			return fmt.Errorf("%s: %w", actor.Name, err)
		}
	}
	return nil
}

var ErrStop = errors.New("stop")
