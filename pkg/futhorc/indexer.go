package futhorc

import (
	"context"
	"slices"
)

type Indexer struct {
	OrderedPosts  <-chan []OrderedPage[Post]
	PageSize      int
	Indices       map[string]*Index
	IndexPages    chan *OrderedPage[IndexPage]
	PageConverter IndexPageConverter
}

func (indexer *Indexer) Run(ctx context.Context) error {
	defer close(indexer.IndexPages)

	// index the posts (NB: there should only be one slice received on the
	// channel, but the indexer could handle many slices)
	for {
		select {
		case <-ctx.Done():
			return ctx.Err()
		case orderedPosts, chanOpen := <-indexer.OrderedPosts:
			if !chanOpen {
				goto PAGINATE
			}

			for i := range orderedPosts {
				p := &orderedPosts[i]
				indexer.fetchIndex("").Push(p)
				for _, tag := range p.Content.Tags {
					indexer.fetchIndex(tag.Text).Push(p)
				}
			}
		}
	}

	// sort and paginate the indices--this will also flush the paginated pages
	// out the `IndexPages` channel.
PAGINATE:
	return indexer.paginate(ctx)
}

func (indexer *Indexer) fetchIndex(id string) *Index {
	if idx, found := indexer.Indices[id]; found {
		return idx
	}
	idx := &Index{ID: id}
	indexer.Indices[id] = idx
	return idx
}

func (indexer *Indexer) paginate(ctx context.Context) error {
	for _, idx := range indexer.Indices {
		if err := indexer.paginateIndex(ctx, idx); err != nil {
			return err
		}
	}
	return nil
}

func (indexer *Indexer) paginateIndex(ctx context.Context, idx *Index) error {
	pages, err := idx.Paginate(indexer.PageSize, &indexer.PageConverter)
	if err != nil {
		return err
	}

	for i := range pages {
		select {
		case <-ctx.Done():
			return ctx.Err()
		case indexer.IndexPages <- &pages[i]:
		}
	}
	return nil
}

type Index struct {
	ID    string
	Posts []*OrderedPage[Post]
}

func (idx *Index) Push(p *OrderedPage[Post]) {
	idx.Posts = append(idx.Posts, p)
}

func (idx *Index) Paginate(
	size int,
	converter *IndexPageConverter,
) (pages []OrderedPage[IndexPage], err error) {
	slices.SortFunc(idx.Posts, func(a, b *OrderedPage[Post]) int {
		return b.Compare(&a.Page)
	})
	for i := range len(idx.Posts) / size {
		pages = append(pages, OrderedPage[IndexPage]{})
		if pages[len(pages)-1].Page, err = converter.Convert(
			idx,
			i,
			i*size,
			(i+1)*size,
		); err != nil {
			return
		}
	}
	if len(idx.Posts)%size > 0 {
		pageNumber := len(idx.Posts) / size
		pages = append(pages, OrderedPage[IndexPage]{})
		if pages[len(pages)-1].Page, err = converter.Convert(
			idx,
			pageNumber,
			// return the remaining posts in this page. Note that while
			// `pageNumber` is defined as the number of posts divided by the
			// page size, `pageNumber*size` is not equal to the number of posts
			// because of rounding during integer division.
			pageNumber*size,
			len(idx.Posts),
		); err != nil {
			return
		}
	}
	OrderPages(pages)
	return
}
