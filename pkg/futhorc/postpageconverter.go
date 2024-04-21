package futhorc

import (
	"futhorc/pkg/markdown"
	"html/template"
	"net/url"
	"strings"
	"time"
)

type PostPageConverter struct {
	Markdown markdown.Config
	PageConverter[Post]
}

func (converter *PostPageConverter) Convert(
	p *Post,
) (content Page[Post], err error) {
	content.Content = *p
	content.Content.Path = convertPath(p.Path)
	if content, err = converter.PageConverter.Convert(
		content.Content.Path,
		time.Time(p.Date).UnixNano(),
		content.Content,
	); err != nil {
		return
	}

	for i := range p.Tags {
		p.Tags[i].URL = template.URL(converter.tagURL(p.Tags[i].Text).String())
	}

	content.Content.Body = markdown.Convert(
		&converter.Markdown,
		content.URL,
		p.Body,
	)
	content.Content.Snippet = snippet(content.Content.Body)
	return
}

func (converter *PostPageConverter) tagURL(tag string) *url.URL {
	return converter.BaseURL.JoinPath(tag, "index.html")
}

func snippet(data template.HTML) template.HTML {
	if idx := strings.Index(string(data), "<!-- more -->"); idx >= 0 {
		return data[:idx]
	} else if idx := strings.Index(string(data), "</p>"); idx >= 0 {
		const max = 1024
		if idx > max {
			idx = max
		}
		return data[:idx]
	}
	return ""
}

func convertPath(p string) string {
	if strings.HasSuffix(p, markdownSuffix) {
		return p[:len(p)-len(markdownSuffix)] + htmlSuffix
	}
	return p
}

type Page[T any] struct {
	Content T
	Order   int64
	Path    string
	URL     *url.URL
}

func (c *Page[T]) Compare(other *Page[T]) int {
	if c.Order < other.Order {
		return -1
	} else if c.Order == other.Order {
		return 0
	} else {
		return 1
	}
}

const markdownSuffix = ".md"
const htmlSuffix = ".html"
