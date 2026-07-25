// Package actor provides small building blocks for running a callback
// repeatedly, optionally with bounded concurrency, until it signals ErrStop.
package actor

import "context"

type Actor interface {
	Run(context.Context) error
}
