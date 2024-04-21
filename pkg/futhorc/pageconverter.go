package futhorc

import (
	"net/url"
	"path/filepath"
)

type PageConverter[T any] struct {
	BaseURL   *url.URL
	Directory string
}

func (converter *PageConverter[T]) Convert(
	path string,
	order int64,
	content T,
) (p Page[T], err error) {
	p.Content = content
	p.Order = order
	p.Path = filepath.Join(converter.Directory, path)
	if p.URL, err = url.Parse(p.Path); err != nil {
		return
	}
	p.URL = converter.BaseURL.ResolveReference(p.URL)
	return
}
