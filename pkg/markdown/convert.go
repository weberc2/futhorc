// Package markdown converts post sources from markdown to HTML via goldmark,
// with build-time syntax highlighting (chroma, CSS-classes mode--no inline
// styles, so themes own the palette and strict CSP stays intact).
package markdown

import (
	"bytes"
	"fmt"
	"html/template"
	"log/slog"
	"net/url"
	"strings"

	chromahtml "github.com/alecthomas/chroma/v2/formatters/html"
	"github.com/yuin/goldmark"
	highlighting "github.com/yuin/goldmark-highlighting/v2"
	gast "github.com/yuin/goldmark/ast"
	"github.com/yuin/goldmark/extension"
	east "github.com/yuin/goldmark/extension/ast"
	"github.com/yuin/goldmark/parser"
	"github.com/yuin/goldmark/renderer"
	"github.com/yuin/goldmark/renderer/html"
	"github.com/yuin/goldmark/text"
	"github.com/yuin/goldmark/util"
)

type Config struct {
	// BaseURL is the root URL of the site; in-site link targets are resolved
	// against it.
	BaseURL *url.URL

	// DeprecateHeadings is the number of levels to demote every heading by
	// (e.g. `2` renders `# foo` as `<h3>`), so post bodies can use `#`
	// without competing with the theme's `<h1>`/`<h2>`.
	DeprecateHeadings uint8
}

// Convert converts a document from markdown to HTML. `url` should be the
// absolute URL for the output document; it's used to convert source urls to
// target urls (and to absolutize footnote reference links so footnotes
// contained in snippets still point to the correct page).
func Convert(c *Config, url *url.URL, doc template.HTML) template.HTML {
	md := goldmark.New(
		goldmark.WithExtensions(
			// parity with the previous gomarkdown CommonExtensions set:
			// tables, strikethrough, autolinks, definition lists, explicit
			// heading ids (`{#id}`, via WithAttribute below), and footnotes.
			extension.Table,
			extension.Strikethrough,
			extension.Linkify,
			extension.DefinitionList,
			extension.Footnote,
			highlighting.NewHighlighting(
				// CSS-classes mode: chroma annotates tokens with classes
				// and emits zero `style` attributes; the theme's stylesheet
				// owns the palette.
				highlighting.WithFormatOptions(chromahtml.WithClasses(true)),
			),
		),
		goldmark.WithParserOptions(
			parser.WithAttribute(),
			parser.WithASTTransformers(util.Prioritized(
				&transformer{Config: c, url: url},
				100,
			)),
		),
		goldmark.WithRendererOptions(
			// raw HTML passthrough: posts embed literal HTML (including the
			// `<!-- more -->` snippet marker) and the input is trusted--it
			// is this site's own source tree.
			html.WithUnsafe(),
			renderer.WithNodeRenderers(util.Prioritized(
				&footnoteLinkRenderer{url: url},
				100,
			)),
		),
	)

	var buf bytes.Buffer
	if err := md.Convert([]byte(doc), &buf); err != nil {
		// goldmark.Convert only fails when the writer fails, and
		// bytes.Buffer writes are infallible.
		panic(fmt.Sprintf("converting markdown for `%s`: %v", url, err))
	}
	return template.HTML(buf.String())
}

// transformer rewrites the parsed AST before rendering: it demotes headings
// and patches link/image destinations (resolving them relative to the site
// and converting in-site markdown-source links into links to the rendered
// HTML).
type transformer struct {
	*Config
	url *url.URL
}

func (t *transformer) Transform(
	doc *gast.Document,
	reader text.Reader,
	pc parser.Context,
) {
	_ = gast.Walk(
		doc,
		func(node gast.Node, entering bool) (gast.WalkStatus, error) {
			if !entering {
				return gast.WalkContinue, nil
			}
			switch node := node.(type) {
			case *gast.Heading:
				node.Level += int(t.DeprecateHeadings)
			case *gast.Link:
				node.Destination = t.patchDestination(node.Destination, true)
			case *gast.Image:
				node.Destination = t.patchDestination(node.Destination, false)
			}
			return gast.WalkContinue, nil
		},
	)
}

// patchDestination resolves a link/image destination the same way the
// previous gomarkdown visitor + render hook did: root-relative paths
// (`/foo`) resolve against the site base URL, everything else resolves
// against the current page URL; then--for links only--in-site links to
// markdown sources (`*.md`) are rewritten to their rendered HTML targets.
func (t *transformer) patchDestination(dst []byte, rewriteMD bool) []byte {
	if len(dst) == 0 {
		return dst
	}
	u, err := url.Parse(string(dst))
	if err != nil {
		slog.Warn(
			"markdown transformer: invalid link url",
			"err", err.Error(),
			"url", string(dst),
		)
		return dst
	}
	resolved := patchURL(t.BaseURL, t.url, u)
	if rewriteMD &&
		strings.HasSuffix(resolved.Path, suffixMarkdown) &&
		isSite(t.BaseURL, resolved) {
		resolved.Path = strings.TrimSuffix(
			resolved.Path,
			suffixMarkdown,
		) + suffixHTML
	}
	return []byte(resolved.String())
}

func patchURL(base, current, u *url.URL) *url.URL {
	if u.Host == "" && u.Scheme == "" && len(u.Path) > 0 && u.Path[0] == '/' {
		return base.JoinPath(u.Path[1:])
	}
	return current.ResolveReference(u)
}

func isSite(baseURL, resolved *url.URL) bool {
	return strings.HasPrefix(resolved.String(), baseURL.String())
}

// footnoteLinkRenderer overrides goldmark's footnote *reference* rendering
// (the `<sup>` link in the body text) to make the href absolute, so
// footnotes contained in snippets still link to the correct page. The
// footnote list at the bottom of the document keeps goldmark's default
// rendering (`<li id="fn:N">`), which the absolute hrefs target.
type footnoteLinkRenderer struct {
	url *url.URL
}

func (r *footnoteLinkRenderer) RegisterFuncs(
	reg renderer.NodeRendererFuncRegisterer,
) {
	reg.Register(east.KindFootnoteLink, r.render)
}

func (r *footnoteLinkRenderer) render(
	w util.BufWriter,
	source []byte,
	node gast.Node,
	entering bool,
) (gast.WalkStatus, error) {
	if entering {
		n := node.(*east.FootnoteLink)
		// mirror goldmark's default id scheme (`fnref[RefIndex]:Index`) so
		// repeated references to one footnote keep unique element ids
		refSuffix := ""
		if n.RefIndex > 0 {
			refSuffix = fmt.Sprintf("%d", n.RefIndex)
		}
		_, _ = fmt.Fprintf(
			w,
			`<sup class="footnote-ref" id="fnref%[3]s:%[2]d">`+
				`<a href="%[1]s#fn:%[2]d">%[2]d</a>`+
				`</sup>`,
			r.url,
			n.Index,
			refSuffix,
		)
	}
	return gast.WalkContinue, nil
}

const (
	suffixMarkdown = ".md"
	suffixHTML     = ".html"
)
