package markdown

import (
	"bytes"
	"fmt"
	"html/template"
	"io"
	"log/slog"
	"net/url"
	"strings"
	"unsafe"

	"github.com/gomarkdown/markdown"
	"github.com/gomarkdown/markdown/ast"
	"github.com/gomarkdown/markdown/html"
	"github.com/gomarkdown/markdown/parser"
)

// Convert converts a document from markdown to HTML. `url` should be the
// absolute path for the output document; it's used to convert source urls to
// target urls.
func Convert(c *Config, url *url.URL, doc template.HTML) template.HTML {
	data := *(*[]byte)(unsafe.Pointer(&doc))
	parser := parser.NewWithExtensions(c.ParserExtensions | parser.Footnotes)
	node := parser.Parse(data)
	ast.Walk(node, &visitor{Config: c, url: url})
	renderer := html.NewRenderer(html.RendererOptions{
		RenderNodeHook: func(
			w io.Writer,
			node ast.Node,
			entering bool,
		) (ast.WalkStatus, bool) {
			// Make footnote links absolute so footnotes contained in snippets
			// still l to the correct page.
			if l, ok := node.(*ast.Link); ok && entering {
				if l.NoteID > 0 {
					w.Write(fmt.Appendf(
						nil,
						`<sup class="footnote-ref" id="fnref:%[2]d">`+
							`<a href="%[1]s#fn:%[2]d">%[2]d</a>`+
							`</sup>`,
						url,
						l.NoteID,
					))
					return ast.SkipChildren, true

					// make sure non-footnote links to other markdown pages in
					// this site are converted into links to the target HTML.
				} else if isSite(c.BaseURL, l.Destination) && isMD(l.Destination) {
					l.Destination = append(
						l.Destination[:len(l.Destination)-len(suffixMarkdown)],
						[]byte(suffixHTML)...,
					)
				}
				return ast.SkipChildren, false
			}
			return ast.GoToNext, false
		},
	})
	tmp := markdown.Render(node, renderer)
	return *(*template.HTML)(unsafe.Pointer(&tmp))
}

func isSite(baseURL *url.URL, target []byte) bool {
	if t, err := url.Parse(*(*string)(unsafe.Pointer(&target))); err == nil {
		resolved := baseURL.ResolveReference(t).String()
		base := baseURL.String()
		return strings.HasPrefix(resolved, base)
	}
	return false
}

func isMD(target []byte) bool {
	return bytes.HasSuffix(target, []byte(suffixMarkdown))
}

type Config struct {
	BaseURL           *url.URL
	ParserExtensions  parser.Extensions
	DeprecateHeadings uint8
}

const CommonExtensions = parser.CommonExtensions

type visitor struct {
	*Config
	url *url.URL
}

func (visitor *visitor) Visit(node ast.Node, entering bool) ast.WalkStatus {
	if heading, ok := node.(*ast.Heading); ok && entering {
		heading.Level += int(visitor.DeprecateHeadings)
		return ast.SkipChildren
	} else if link, ok := node.(*ast.Link); ok && entering {
		if len(link.Destination) > 0 {
			dst, err := url.Parse(*(*string)(unsafe.Pointer(&link.Destination)))
			if err != nil {
				slog.Warn(
					"DocumentVisitor: invalid link url",
					"err", err.Error(),
					"url", string(link.Destination),
				)
				return ast.SkipChildren
			}
			resolved := patchURL(visitor.BaseURL, visitor.url, dst).String()
			link.Destination = *(*[]byte)(unsafe.Pointer(&resolved))
			return ast.SkipChildren
		}
	}
	return ast.GoToNext
}

func patchURL(base, current, u *url.URL) *url.URL {
	if u.Host == "" && u.Scheme == "" && len(u.Path) > 0 && u.Path[0] == '/' {
		return base.JoinPath(u.Path[1:])
	}
	return current.ResolveReference(u)
}

const (
	suffixMarkdown = ".md"
	suffixHTML     = ".html"
)
