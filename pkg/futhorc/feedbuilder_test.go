package futhorc

import (
	"encoding/json"
	"fmt"
	"html/template"
	"net/url"
	"testing"
	"time"

	"github.com/go-git/go-billy/v5/memfs"
	"github.com/go-git/go-billy/v5/util"
	"github.com/gorilla/feeds"
)

// jsonFeedVersion is the version URL required by the JSON Feed v1.1 spec
// (https://www.jsonfeed.org/version/1.1/, "version (required, string)").
const jsonFeedVersion = "https://jsonfeed.org/version/1.1"

// jsonFeedDoc models the subset of a JSON Feed v1.1 document that the tests
// assert on.
type jsonFeedDoc struct {
	Version     string         `json:"version"`
	Title       string         `json:"title"`
	HomePageURL string         `json:"home_page_url"`
	FeedURL     string         `json:"feed_url"`
	NextURL     string         `json:"next_url"`
	Items       []jsonFeedItem `json:"items"`
}

type jsonFeedItem struct {
	ID            string `json:"id"`
	URL           string `json:"url"`
	Title         string `json:"title"`
	ContentHTML   string `json:"content_html"`
	ContentText   string `json:"content_text"`
	Summary       string `json:"summary"`
	DatePublished string `json:"date_published"`
}

func TestBuildFeed(t *testing.T) {
	baseURL, err := url.Parse("https://example.com/")
	if err != nil {
		t.Fatalf("parsing base url: %v", err)
	}

	posts := []*OrderedPage[Post]{
		testFeedPost(t, baseURL, "second-post", "Second Post", 2024, 2),
		testFeedPost(t, baseURL, "first-post", "First Post", 2024, 1),
	}

	for _, testCase := range []struct {
		name string
		page OrderedPage[IndexPage]

		// wantedPath is the path of the feed file expected in the output
		// filesystem; empty means no file must be written.
		wantedPath    string
		wantedFeedURL string
		wantedNextURL string
	}{
		{
			name: "single-page",
			page: OrderedPage[IndexPage]{
				Page: testIndexPage(t, baseURL, "index.html", posts),
			},
			wantedPath:    "index.json",
			wantedFeedURL: "https://example.com/index.json",
			wantedNextURL: "",
		},
		{
			name: "paginated-next-url-points-at-feed",
			page: OrderedPage[IndexPage]{
				Page: testIndexPage(t, baseURL, "index.html", posts),
				Next: baseURL.JoinPath("page-001.html"),
			},
			wantedPath:    "index.json",
			wantedFeedURL: "https://example.com/index.json",
			wantedNextURL: "https://example.com/page-001.json",
		},
		{
			name: "tag-index-skipped",
			page: OrderedPage[IndexPage]{
				Page: func() Page[IndexPage] {
					page := testIndexPage(
						t,
						baseURL,
						"testing/index.html",
						posts,
					)
					page.Content.IndexID = "testing"
					return page
				}(),
			},
			wantedPath: "",
		},
	} {
		t.Run(testCase.name, func(t *testing.T) {
			output := memfs.New()
			header := feeds.Feed{
				Title:       "Test Feed",
				Description: "A test feed",
				Author:      &feeds.Author{Name: "Test Author"},
			}

			if err := buildFeed(&header, output, &testCase.page); err != nil {
				t.Fatalf("unexpected error: %v", err)
			}

			if testCase.wantedPath == "" {
				if entries, err := output.ReadDir("."); err == nil &&
					len(entries) > 0 {
					t.Fatalf(
						"wanted no feed output; found %d entries",
						len(entries),
					)
				}
				return
			}

			data, err := util.ReadFile(output, testCase.wantedPath)
			if err != nil {
				t.Fatalf(
					"reading feed file `%s`: %v",
					testCase.wantedPath,
					err,
				)
			}

			feed := assertValidJSONFeed(t, data)

			if feed.FeedURL != testCase.wantedFeedURL {
				t.Errorf(
					"wanted feed_url `%s`; found `%s`",
					testCase.wantedFeedURL,
					feed.FeedURL,
				)
			}

			if feed.NextURL != testCase.wantedNextURL {
				t.Errorf(
					"wanted next_url `%s`; found `%s`",
					testCase.wantedNextURL,
					feed.NextURL,
				)
			}

			if len(feed.Items) != len(posts) {
				t.Fatalf(
					"wanted %d items; found %d",
					len(posts),
					len(feed.Items),
				)
			}

			for i, item := range feed.Items {
				post := &posts[i].Page
				if wanted := post.URL.String(); item.ID != wanted {
					t.Errorf(
						"items[%d]: wanted id `%s`; found `%s`",
						i,
						wanted,
						item.ID,
					)
				}
				if wanted := post.Content.Title; item.Title != wanted {
					t.Errorf(
						"items[%d]: wanted title `%s`; found `%s`",
						i,
						wanted,
						item.Title,
					)
				}
				if wanted := string(
					post.Content.Body,
				); item.ContentHTML != wanted {
					t.Errorf(
						"items[%d]: wanted content_html `%s`; found `%s`",
						i,
						wanted,
						item.ContentHTML,
					)
				}
				if wanted := string(
					post.Content.Snippet,
				); item.Summary != wanted {
					t.Errorf(
						"items[%d]: wanted summary `%s`; found `%s`",
						i,
						wanted,
						item.Summary,
					)
				}
			}
		})
	}
}

// assertValidJSONFeed asserts the properties the JSON Feed v1.1 spec
// (https://www.jsonfeed.org/version/1.1/) requires of every feed document:
// valid JSON, the exact v1.1 `version` URL, a `title`, and--for every
// item--a unique non-empty `id`, at least one of
// `content_html`/`content_text`, and an RFC3339 `date_published`. It also
// asserts the spec's "should" that item `url`s are absolute.
func assertValidJSONFeed(t *testing.T, data []byte) jsonFeedDoc {
	t.Helper()

	if !json.Valid(data) {
		t.Fatalf("feed is not valid JSON: %s", data)
	}

	var feed jsonFeedDoc
	if err := json.Unmarshal(data, &feed); err != nil {
		t.Fatalf("unmarshaling feed: %v", err)
	}

	if feed.Version != jsonFeedVersion {
		t.Errorf(
			"wanted version `%s`; found `%s`",
			jsonFeedVersion,
			feed.Version,
		)
	}

	if feed.Title == "" {
		t.Errorf("missing required feed `title`")
	}

	ids := make(map[string]bool, len(feed.Items))
	for i, item := range feed.Items {
		if item.ID == "" {
			t.Errorf("items[%d]: missing required `id`", i)
		}
		if ids[item.ID] {
			t.Errorf("items[%d]: duplicate id `%s`", i, item.ID)
		}
		ids[item.ID] = true

		if item.ContentHTML == "" && item.ContentText == "" {
			t.Errorf(
				"items[%d]: neither `content_html` nor `content_text` is "+
					"present",
				i,
			)
		}

		if item.DatePublished == "" {
			t.Errorf("items[%d]: missing `date_published`", i)
		} else if _, err := time.Parse(
			time.RFC3339,
			item.DatePublished,
		); err != nil {
			t.Errorf(
				"items[%d]: `date_published` is not RFC3339: %v",
				i,
				err,
			)
		}

		if item.URL == "" {
			t.Errorf("items[%d]: missing `url`", i)
		} else if u, err := url.Parse(item.URL); err != nil {
			t.Errorf("items[%d]: invalid `url`: %v", i, err)
		} else if !u.IsAbs() {
			t.Errorf("items[%d]: `url` is not absolute: `%s`", i, item.URL)
		}
	}

	return feed
}

func testFeedPost(
	t *testing.T,
	baseURL *url.URL,
	slug string,
	title string,
	year int,
	month time.Month,
) *OrderedPage[Post] {
	t.Helper()
	path := "posts/" + slug + ".html"
	rel, err := url.Parse(path)
	if err != nil {
		t.Fatalf("parsing post url `%s`: %v", path, err)
	}
	date := time.Date(year, month, 1, 0, 0, 0, 0, time.UTC)
	return &OrderedPage[Post]{
		Page: Page[Post]{
			Content: Post{
				Frontmatter: Frontmatter{
					Title:  title,
					Author: "Test Author",
					Date:   Date(date),
				},
				Path: path,
				Body: template.HTML(
					fmt.Sprintf("<p>%s body</p>", title),
				),
				Snippet: template.HTML(
					fmt.Sprintf("<p>%s snippet</p>", title),
				),
			},
			Order: date.UnixNano(),
			Path:  path,
			URL:   baseURL.ResolveReference(rel),
		},
	}
}

func testIndexPage(
	t *testing.T,
	baseURL *url.URL,
	path string,
	posts []*OrderedPage[Post],
) Page[IndexPage] {
	t.Helper()
	rel, err := url.Parse(path)
	if err != nil {
		t.Fatalf("parsing index url `%s`: %v", path, err)
	}
	return Page[IndexPage]{
		Content: IndexPage{Posts: posts},
		Path:    path,
		URL:     baseURL.ResolveReference(rel),
	}
}
