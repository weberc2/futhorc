package actor

import "context"

type Multi []Actor

func (actors Multi) Run(ctx context.Context) error {
	ctx, cancel := context.WithCancel(ctx)
	defer cancel()
	results := make(chan error)
	for _, a := range actors {
		go func(actor Actor) { results <- actor.Run(ctx) }(a)
	}

	for range actors {
		if err := <-results; err != nil {
			return err
		}
	}
	return nil
}
