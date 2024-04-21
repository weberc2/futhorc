package actor

import (
	"context"
)

type Input[T any] struct {
	Base
	Input <-chan T
}

type InputCallback[T any] func(ctx context.Context, elt T) error

func NewInput[T any](
	name string,
	concurrency int,
	input <-chan T,
	callback InputCallback[T],
	done func(ctx context.Context) error,
) (actor Input[T]) {
	actor.Name = name
	actor.Concurrency = concurrency
	actor.Input = input
	actor.Callback = func(ctx context.Context) error {
		select {
		case <-ctx.Done():
			return ctx.Err()
		case elt, ok := <-actor.Input:
			if !ok {
				if done != nil {
					if err := done(ctx); err != nil {
						return err
					}
				}
				return ErrStop
			}
			if err := callback(ctx, elt); err != nil {
				return err
			}
			return nil
		}
	}
	return
}
