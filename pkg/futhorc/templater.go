package futhorc

import (
	"context"
	"errors"
	"fmt"
	"futhorc/pkg/actor"
	"html/template"

	"github.com/go-git/go-billy/v5"
)

type Templater[T any] actor.Input[*OrderedPage[T]]

type SiteData struct {
	BaseURL     template.URL
	HomePage    template.URL
	FeedURL     template.URL
	FeedType    string
	ThemeAssets template.URL
}

type TemplaterParams[T any] struct {
	Name        string
	Concurrency int
	Pages       <-chan *OrderedPage[T]
	Output      billy.Filesystem
	Template    *template.Template
	SiteData    *SiteData
}

func NewTemplater[T any](params *TemplaterParams[T]) (templater Templater[T]) {
	if params.Name == "" {
		params.Name = fmt.Sprintf("%T", templater)
	}
	templater = Templater[T](actor.NewInput(
		params.Name,
		params.Concurrency,
		params.Pages,
		func(ctx context.Context, page *OrderedPage[T]) error {
			t := params.Template
			if err := exec(
				params.Output,
				page.Path,
				t,
				struct {
					*SiteData
					*OrderedPage[T]
				}{
					SiteData:    params.SiteData,
					OrderedPage: page,
				},
			); err != nil {
				return fmt.Errorf("transforming post `%s`: %w'", page.Path, err)
			}
			return nil
		},
		nil,
	))
	return
}

func exec(
	fs billy.Filesystem,
	path string,
	t *template.Template,
	v any,
) (err error) {
	var f billy.File
	f, err = fs.Create(path)
	if err != nil {
		return
	}
	defer func() { err = errors.Join(err, f.Close()) }()
	err = t.Execute(f, v)
	return
}
