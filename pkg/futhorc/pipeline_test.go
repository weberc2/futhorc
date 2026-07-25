package futhorc

import (
	"context"
	"html/template"
	"net/url"
	"os"
	"strings"
	"testing"
	"time"

	"github.com/go-git/go-billy/v5"
	"github.com/go-git/go-billy/v5/osfs"
	"github.com/go-git/go-billy/v5/util"
)

// testPipeline builds a Pipeline over the `testdata/site` fixture, mirroring
// `LoadPipeline` except that output goes to a per-test temporary directory.
func testPipeline(t *testing.T, baseURL *url.URL) Pipeline {
	t.Helper()

	theme, err := LoadTheme(os.DirFS("testdata/site/theme"))
	if err != nil {
		t.Fatalf("loading theme: %v", err)
	}

	return Pipeline{
		PostSources:   os.DirFS("testdata/site/posts"),
		ThemeAssets:   os.DirFS("testdata/site/theme/assets"),
		BaseURL:       baseURL,
		PostTemplate:  theme.PostTemplate,
		IndexTemplate: theme.IndexTemplate,
		SiteData: SiteData{
			BaseURL: template.URL(baseURL.String()),
			HomePage: template.URL(
				baseURL.JoinPath("index.html").String(),
			),
			ThemeAssets: template.URL(
				baseURL.JoinPath("assets/theme/").String(),
			),
			FeedURL: template.URL(
				baseURL.JoinPath("index.json").String(),
			),
			FeedType: "application/json",
		},
		OutputDirectory: osfs.New(t.TempDir()),
	}
}

func TestPipeline(t *testing.T) {
	baseURL, err := url.Parse("https://example.com/")
	if err != nil {
		t.Fatalf("parsing base url: %v", err)
	}

	pipeline := testPipeline(t, baseURL)

	ctx, cancel := context.WithTimeout(context.Background(), time.Minute)
	defer cancel()
	if err := pipeline.Run(ctx); err != nil {
		t.Fatalf("running pipeline: %v", err)
	}

	output := pipeline.OutputDirectory

	// every expected output file exists
	for _, path := range []string{
		"index.html",
		"index.json",
		"posts/first-post.html",
		"posts/second-post.html",
		"posts/third-post.html",
		"testing/index.html",
		"golang/index.html",
		"assets/theme/theme.css",
		"assets/posts/pixel.svg",
	} {
		if _, err := output.Stat(path); err != nil {
			t.Errorf("missing output file `%s`: %v", path, err)
		}
	}

	// tag indices must not produce feeds
	for _, path := range []string{"testing/index.json", "golang/index.json"} {
		if _, err := output.Stat(path); err == nil {
			t.Errorf("unexpected feed for tag index: `%s`", path)
		}
	}

	t.Run("Feed", func(t *testing.T) {
		data, err := util.ReadFile(output, "index.json")
		if err != nil {
			t.Fatalf("reading feed: %v", err)
		}

		feed := assertValidJSONFeed(t, data)

		if wanted := "https://example.com/index.json"; feed.FeedURL != wanted {
			t.Errorf("wanted feed_url `%s`; found `%s`", wanted, feed.FeedURL)
		}

		// 3 fixture posts fit on one index page: no next_url
		if feed.NextURL != "" {
			t.Errorf("wanted no next_url; found `%s`", feed.NextURL)
		}

		// items are ordered newest-first and ids are the canonical post URLs
		wantedIDs := []string{
			"https://example.com/posts/third-post.html",
			"https://example.com/posts/second-post.html",
			"https://example.com/posts/first-post.html",
		}
		if len(feed.Items) != len(wantedIDs) {
			t.Fatalf(
				"wanted %d items; found %d",
				len(wantedIDs),
				len(feed.Items),
			)
		}
		for i, wanted := range wantedIDs {
			if feed.Items[i].ID != wanted {
				t.Errorf(
					"items[%d]: wanted id `%s`; found `%s`",
					i,
					wanted,
					feed.Items[i].ID,
				)
			}
			if feed.Items[i].URL != wanted {
				t.Errorf(
					"items[%d]: wanted url `%s`; found `%s`",
					i,
					wanted,
					feed.Items[i].URL,
				)
			}
		}

		if wanted := "2024-03-01T00:00:00Z"; feed.Items[0].DatePublished != wanted {
			t.Errorf(
				"items[0]: wanted date_published `%s`; found `%s`",
				wanted,
				feed.Items[0].DatePublished,
			)
		}

		// content_html carries the converted post body, with in-site
		// markdown links rewritten to their HTML targets
		firstPost := feed.Items[2]
		if !strings.Contains(
			firstPost.ContentHTML,
			`href="https://example.com/posts/second-post.html"`,
		) {
			t.Errorf(
				"items[2]: content_html should link to "+
					"`second-post.html`; found: %s",
				firstPost.ContentHTML,
			)
		}
		if strings.Contains(firstPost.ContentHTML, ".md") {
			t.Errorf(
				"items[2]: content_html still links to a markdown source: "+
					"%s",
				firstPost.ContentHTML,
			)
		}
	})

	t.Run("PostHTML", func(t *testing.T) {
		html := readOutput(t, output, "posts/first-post.html")

		// links to in-site markdown sources are rewritten to their HTML
		// targets: `./second-post.md` (relative) and
		// `/posts/third-post.md` (absolute path)
		for _, wanted := range []string{
			`href="https://example.com/posts/second-post.html"`,
			`href="https://example.com/posts/third-post.html"`,
			`<h1><a href="https://example.com/posts/first-post.html">` +
				`First Post</a></h1>`,
		} {
			if !strings.Contains(html, wanted) {
				t.Errorf("wanted `%s` in output; found: %s", wanted, html)
			}
		}
	})

	t.Run("Highlighting", func(t *testing.T) {
		html := readOutput(t, output, "posts/second-post.html")

		// build-time syntax highlighting: chroma annotates the fenced go
		// block with classes only--no inline styles (the theme stylesheet
		// owns the palette; strict CSP stays intact)
		for _, wanted := range []string{
			`<pre class="chroma">`,
			`<span class="kn">package</span>`,
		} {
			if !strings.Contains(html, wanted) {
				t.Errorf("wanted `%s` in output; found: %s", wanted, html)
			}
		}
		if strings.Contains(html, "style=") {
			t.Errorf("post HTML contains inline styles: %s", html)
		}
	})

	t.Run("IndexHTML", func(t *testing.T) {
		html := readOutput(t, output, "index.html")

		// snippets end at the `<!-- more -->` marker: the post body after
		// the marker must not leak into the index page
		if strings.Contains(html, "the rest of the first post") {
			t.Errorf("index page contains post body past the snippet marker")
		}

		// newest-first ordering of the three posts
		third := strings.Index(html, "Third Post")
		second := strings.Index(html, "Second Post")
		first := strings.Index(html, "First Post")
		if third == -1 || second == -1 || first == -1 {
			t.Fatalf("index page is missing post titles: %s", html)
		}
		if third >= second || second >= first {
			t.Errorf(
				"index page posts are not newest-first: positions "+
					"third=%d second=%d first=%d",
				third,
				second,
				first,
			)
		}
	})

	t.Run("TagIndexHTML", func(t *testing.T) {
		html := readOutput(t, output, "golang/index.html")
		if strings.Contains(html, "First Post") ||
			strings.Contains(html, "Third Post") {
			t.Errorf(
				"golang tag index should only contain Second Post; "+
					"found: %s",
				html,
			)
		}
		if !strings.Contains(html, "Second Post") {
			t.Errorf("golang tag index is missing Second Post: %s", html)
		}
	})
}

func readOutput(t *testing.T, fs billy.Filesystem, path string) string {
	t.Helper()
	data, err := util.ReadFile(fs, path)
	if err != nil {
		t.Fatalf("reading output file `%s`: %v", path, err)
	}
	return string(data)
}
