package futhorc

import (
	"context"
	"log/slog"
)

type MultiChan[T any] struct {
	Input   <-chan T
	Outputs []chan T
}

func (ch *MultiChan[T]) Output(i int) <-chan T {
	return ch.Outputs[i]
}

func (ch *MultiChan[T]) Run(ctx context.Context) error {
	defer slog.Debug("closing actor", "name", "MultiChan")
	defer func() {
		for _, out := range ch.Outputs {
			close(out)
		}
	}()

	for {
		select {
		case <-ctx.Done():
			return ctx.Err()
		case elt, chanOpen := <-ch.Input:
			if !chanOpen {
				return nil
			}

			for _, out := range ch.Outputs {
				select {
				case <-ctx.Done():
					return ctx.Err()
				default:
					out <- elt
				}
			}
		}
	}
}
