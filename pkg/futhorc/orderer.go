package futhorc

import (
	"context"
	"futhorc/pkg/actor"
	"net/url"
	"slices"
)

type Orderer[T any] struct {
	actor.Input[Page[T]]
	OrderedPages      chan *OrderedPage[T]
	OrderedPageSlices chan []OrderedPage[T]
}

func NewOrderer[T any](
	name string,
	pages <-chan Page[T],
) (orderer Orderer[T]) {
	var orderedPages []OrderedPage[T]
	orderer.OrderedPages = make(chan *OrderedPage[T])
	orderer.OrderedPageSlices = make(chan []OrderedPage[T])
	orderer.Input = actor.NewInput[Page[T]](
		name,
		1,
		pages,
		func(ctx context.Context, page Page[T]) error {
			orderedPages = append(
				orderedPages,
				OrderedPage[T]{Page: page},
			)
			return nil
		},
		func(ctx context.Context) error {
			OrderPages(orderedPages)

			orderer.OrderedPageSlices <- orderedPages

			for i := range orderedPages {
				orderer.OrderedPages <- &orderedPages[i]
			}
			return nil
		},
	)
	return
}

func (orderer *Orderer[T]) Run(ctx context.Context) error {
	defer close(orderer.OrderedPages)
	defer close(orderer.OrderedPageSlices)
	return orderer.Input.Run(ctx)
}

func OrderPages[T any](orderedPages []OrderedPage[T]) {
	if len(orderedPages) < 1 {
		return
	}

	slices.SortFunc(orderedPages, func(a, b OrderedPage[T]) int {
		return b.Compare(&a.Page)
	})

	for i := range orderedPages[1:] {
		orderedPages[i].Prev = orderedPages[i+1].URL
	}

	for i := range orderedPages[:len(orderedPages)-1] {
		orderedPages[i+1].Next = orderedPages[i].URL
	}
}

type OrderedPage[T any] struct {
	Page[T]
	Next *url.URL
	Prev *url.URL
}
