package actor

import (
	"context"
)

type Output[T any] struct {
	Base
	Output chan T
}

type OutputCallback[T any] func(context.Context) (T, error)

func NewOutput[T any](
	name string,
	concurrency int,
	callback OutputCallback[T],
) (actor Output[T]) {
	actor.Name = name
	actor.Concurrency = concurrency
	actor.Callback = func(ctx context.Context) error {
		out, err := callback(ctx)
		if err != nil {
			return err
		}
		actor.Output <- out
		return nil
	}
	actor.Output = make(chan T)
	return
}

func (actor *Output[T]) Run(ctx context.Context) error {
	defer close(actor.Output)
	return actor.Base.Run(ctx)
}

func (actor *Output[T]) OutputChan() <-chan T {
	return actor.Output
}
