package actor

import (
	"context"
)

type Map[I, O any] struct {
	Input[I]
	Output chan O
}

func NewMap[I, O any](
	name string,
	concurrency int,
	input <-chan I,
	callback func(context.Context, I) (O, error),
) (actor Map[I, O]) {
	actor.Input = NewInput(
		name,
		concurrency,
		input,
		func(ctx context.Context, elt I) error {
			out, err := callback(ctx, elt)
			if err != nil {
				return err
			}
			actor.Output <- out
			return nil
		},
		nil,
	)
	actor.Output = make(chan O)
	return
}

func (actor *Map[I, O]) Run(ctx context.Context) error {
	defer close(actor.Output)
	return actor.Base.Run(ctx)
}
