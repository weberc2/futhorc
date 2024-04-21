package futhorc

import (
	"context"
	"encoding/json"
	"errors"
	"fmt"
	"time"
	"unsafe"

	"futhorc/pkg/actor"

	"github.com/go-git/go-billy/v5"
	"github.com/gorilla/feeds"
)

func FeedBuilder(
	header *feeds.Feed,
	output billy.Filesystem,
) actor.InputCallback[*OrderedPage[IndexPage]] {
	return func(ctx context.Context, page *OrderedPage[IndexPage]) error {
		return buildFeed(header, output, page)
	}
}

func buildFeed(
	header *feeds.Feed,
	output billy.Filesystem,
	page *OrderedPage[IndexPage],
) error {
	// skip tag indices
	if page.Content.IndexID != "" {
		return nil
	}
	path := page.Path[:len(page.Path)-len(htmlSuffix)] + jsonSuffix

	feed := buildFeedPage(header, &page.Page)
	var next string
	if page.Next != nil {
		next = page.Next.String()
	}
	data, err := json.Marshal(struct {
		*feeds.JSONFeed
		Next string `json:"next_url,omitempty"`
	}{
		JSONFeed: (&feeds.JSON{Feed: &feed}).JSONFeed(),
		Next:     next,
	})
	if err != nil {
		return fmt.Errorf("rendering feed for index page `%s`: %w", path, err)
	}

	file, err := output.Create(path)
	if err != nil {
		return fmt.Errorf("rendering feed for index page `%s`: %w", path, err)
	}

	if _, err := file.Write(data); err != nil {
		return fmt.Errorf(
			"rendering feed for index page `%s`: %w",
			path,
			errors.Join(err, file.Close()),
		)
	}

	if err := file.Close(); err != nil {
		return fmt.Errorf("rendering feed for index page `%s`: %w", path, err)
	}

	return nil
}

func buildFeedPage(
	header *feeds.Feed,
	page *Page[IndexPage],
) (feed feeds.Feed) {
	type item struct {
		item   feeds.Item
		author feeds.Author
		link   feeds.Link
	}
	items := make([]item, len(page.Content.Posts))

	feed = *header
	feed.Link = &feeds.Link{Href: page.URL.String()}
	feed.Items = make([]*feeds.Item, len(items))
	for i := range items {
		items[i].item = buildFeedItem(
			&page.Content.Posts[i].Page,
			&items[i].author,
			&items[i].link,
		)
		feed.Items[i] = &items[i].item
	}
	return
}

func buildFeedItem(
	p *Page[Post],
	author *feeds.Author,
	link *feeds.Link,
) (item feeds.Item) {
	author.Name = p.Content.Author
	item.Author = author
	item.Created = time.Time(p.Content.Date)
	link.Href = p.URL.String()
	item.Link = link
	item.Description = *(*string)(unsafe.Pointer(&p.Content.Snippet))
	return
}

const jsonSuffix = ".json"
