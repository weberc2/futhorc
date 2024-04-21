package actor

import "context"

type Actor interface {
	Run(context.Context) error
}
