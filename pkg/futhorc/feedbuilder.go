package futhorc

import (
	"context"
	"encoding/json"
	"errors"
	"fmt"
	"net/url"
	"strings"
	"time"

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
	path := feedPath(page.Path)

	feed := buildFeedPage(header, &page.Page)
	jsonFeed := (&feeds.JSON{Feed: &feed}).JSONFeed()

	// `feed_url` should be the canonical URL of the feed document itself
	// (JSON Feed v1.1, top-level `feed_url`), and `next_url` must point at
	// the *feed* for the next page of items--not at the next index page's
	// HTML rendering.
	jsonFeed.FeedUrl = feedURL(page.URL)
	if page.Next != nil {
		jsonFeed.NextUrl = feedURL(page.Next)
	}

	data, err := json.Marshal(jsonFeed)
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
	item.Title = p.Content.Title
	item.Created = time.Time(p.Content.Date)
	link.Href = p.URL.String()
	item.Link = link

	// JSON Feed v1.1 requires a unique `id` per item ("id (required,
	// string)"); the post's canonical URL is the conventional choice.
	item.Id = p.URL.String()

	// JSON Feed v1.1 requires one or both of `content_html`/`content_text`;
	// `feeds.Item.Content` maps to `content_html`. The snippet maps to the
	// optional `summary` field.
	item.Content = string(p.Content.Body)
	item.Description = string(p.Content.Snippet)
	return
}

// feedPath converts an index page's output path (`*.html`) into the output
// path of the feed document describing that page (`*.json`).
func feedPath(indexPath string) string {
	return strings.TrimSuffix(indexPath, htmlSuffix) + jsonSuffix
}

// feedURL converts an index page's URL into the URL of the feed document
// describing that page.
func feedURL(indexURL *url.URL) string {
	u := *indexURL
	u.Path = strings.TrimSuffix(u.Path, htmlSuffix) + jsonSuffix
	return u.String()
}

const jsonSuffix = ".json"
