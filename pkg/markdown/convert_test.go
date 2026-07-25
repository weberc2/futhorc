package markdown

import (
	"html/template"
	"net/url"
	"strings"
	"testing"
)

func TestConvert(t *testing.T) {
	baseURL, err := url.Parse("https://example.com/")
	if err != nil {
		t.Fatalf("parsing base url: %v", err)
	}
	pageURL, err := url.Parse("https://example.com/posts/test-post.html")
	if err != nil {
		t.Fatalf("parsing page url: %v", err)
	}

	for _, testCase := range []struct {
		name              string
		deprecateHeadings uint8
		input             string

		// wanted substrings must all appear in the output; unwanted
		// substrings must not appear.
		wanted   []string
		unwanted []string
	}{
		{
			name:  "fenced-code-blocks-are-highlighted",
			input: "```go\npackage main\n\nfunc main() {}\n```\n",
			wanted: []string{
				`<pre class="chroma"><code>`,
				`<span class="kn">package</span>`,
				`<span class="kd">func</span>`,
			},
			// CSS-classes mode: the palette belongs to the theme
			// stylesheet, so no inline styles may appear (CSP-strict).
			unwanted: []string{`style=`},
		},
		{
			name:  "yaml-fences-are-highlighted",
			input: "```yaml\nkey: value\n```\n",
			wanted: []string{
				`<pre class="chroma"><code>`,
				`<span class="nt">key</span>`,
			},
			unwanted: []string{`style=`},
		},
		{
			// regression test for the upstream gomarkdown defect
			// (https://github.com/gomarkdown/markdown/issues/276): a fenced
			// code block nested in an HTML block was emitted literally
			// (backticks and all) instead of being rendered. Per CommonMark,
			// the blank line ends the HTML block, so the fence must be
			// parsed as markdown again.
			name: "fence-inside-html-block-is-rendered",
			input: "<figure>\n\n```go\npackage main\n```\n\n" +
				"<figcaption>caption</figcaption>\n</figure>\n",
			wanted: []string{
				"<figure>",
				`<pre class="chroma"><code>`,
				`<span class="kn">package</span>`,
				"<figcaption>caption</figcaption>",
				"</figure>",
			},
			unwanted: []string{"```"},
		},
		{
			name: "raw-html-passes-through",
			input: "before\n\n<!-- more -->\n\n" +
				"<div class=\"aside\">raw <b>html</b></div>\n",
			wanted: []string{
				"<!-- more -->",
				`<div class="aside">raw <b>html</b></div>`,
			},
		},
		{
			name:              "headings-are-demoted",
			deprecateHeadings: 2,
			input:             "# Top\n\n### Nested\n",
			wanted:            []string{"<h3>Top</h3>", "<h5>Nested</h5>"},
			unwanted:          []string{"<h1>", "<h4>"},
		},
		{
			name:   "explicit-heading-ids",
			input:  "## Section {#custom-id}\n",
			wanted: []string{`id="custom-id"`},
		},
		{
			name:  "in-site-markdown-links-are-rewritten",
			input: "[relative](./other-post.md) and [rooted](/posts/deep/post.md)\n",
			wanted: []string{
				`href="https://example.com/posts/other-post.html"`,
				`href="https://example.com/posts/deep/post.html"`,
			},
			unwanted: []string{".md"},
		},
		{
			name:  "external-markdown-links-are-untouched",
			input: "[external](https://other.example.org/doc.md)\n",
			wanted: []string{
				`href="https://other.example.org/doc.md"`,
			},
		},
		{
			name:  "image-urls-are-resolved",
			input: "![rooted](/assets/pic.png) ![relative](./pic2.png)\n",
			wanted: []string{
				`src="https://example.com/assets/pic.png"`,
				`src="https://example.com/posts/pic2.png"`,
			},
		},
		{
			name:  "footnote-refs-are-absolute",
			input: "body text[^note]\n\n[^note]: the note\n",
			wanted: []string{
				// the reference links to the footnote on the *post* page,
				// so it keeps working when the snippet is embedded in an
				// index page
				`<sup class="footnote-ref" id="fnref:1">` +
					`<a href="https://example.com/posts/test-post.html#fn:1">` +
					`1</a></sup>`,
				// ...and the target exists in the footnote list
				`<li id="fn:1">`,
			},
		},
		{
			name: "tables-and-strikethrough",
			input: "| a | b |\n|---|---|\n| 1 | 2 |\n\n" +
				"~~struck~~\n",
			wanted: []string{"<table>", "<del>struck</del>"},
		},
	} {
		t.Run(testCase.name, func(t *testing.T) {
			output := string(Convert(
				&Config{
					BaseURL:           baseURL,
					DeprecateHeadings: testCase.deprecateHeadings,
				},
				pageURL,
				template.HTML(testCase.input),
			))

			for _, wanted := range testCase.wanted {
				if !strings.Contains(output, wanted) {
					t.Errorf(
						"wanted `%s` in output; found:\n%s",
						wanted,
						output,
					)
				}
			}
			for _, unwanted := range testCase.unwanted {
				if strings.Contains(output, unwanted) {
					t.Errorf(
						"unwanted `%s` in output; found:\n%s",
						unwanted,
						output,
					)
				}
			}
		})
	}
}
