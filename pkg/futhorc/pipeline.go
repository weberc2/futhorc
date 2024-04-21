package futhorc

import (
	"context"
	"futhorc/pkg/actor"
	"futhorc/pkg/markdown"
	"html/template"
	"io/fs"
	"net/url"
	"runtime/trace"
	"time"

	"github.com/go-git/go-billy/v5"
	"github.com/gorilla/feeds"
)

type Pipeline struct {
	PostSources                fs.FS
	PostAssets                 fs.FS
	ThemeAssets                fs.FS
	BaseURL                    *url.URL
	SiteData                   SiteData
	PostTemplate               *template.Template
	IndexTemplate              *template.Template
	OutputDirectory            billy.Filesystem
	OutputDirectoryPostAssets  billy.Filesystem
	OutputDirectoryThemeAssets billy.Filesystem
}

func (pipeline *Pipeline) Run(ctx context.Context) error {
	region := trace.StartRegion(ctx, "pipeline")
	defer region.End()
	ctx, task := trace.NewTask(ctx, "pipeline")
	defer task.End()

	themeAssetsFinder := actor.NewOutput(
		"FileFinder::ThemeAssets",
		1,
		FileFinder(pipeline.ThemeAssets, ""),
	)

	themeAssetsCopier := actor.NewInput(
		"FileCopier::ThemeAssets",
		4,
		themeAssetsFinder.OutputChan(),
		FileCopier(pipeline.OutputDirectoryThemeAssets, pipeline.ThemeAssets),
		nil,
	)

	sourceFinder := actor.NewOutput(
		"FileFinder::PostSources",
		1,
		FileFinder(pipeline.PostSources, markdownSuffix),
	)

	sourceReader := NewFileReader(
		"FileReader",
		4,
		pipeline.PostSources,
		sourceFinder.OutputChan(),
	)

	parser := NewPostParser(
		"PostParser",
		8,
		sourceReader.Output(),
		&PostPageConverter{
			Markdown: markdown.Config{
				BaseURL:           pipeline.BaseURL,
				ParserExtensions:  markdown.CommonExtensions,
				DeprecateHeadings: 2,
			},
			PageConverter: PageConverter[Post]{
				BaseURL:   pipeline.BaseURL,
				Directory: "posts",
			},
		},
	)

	orderer := NewOrderer("Orderer", parser.Output)

	postTemplater := NewTemplater(&TemplaterParams[Post]{
		Name:        "Templater[Post]",
		Concurrency: 8,
		Pages:       orderer.OrderedPages,
		Output:      pipeline.OutputDirectory,
		Template:    pipeline.PostTemplate,
		SiteData:    &pipeline.SiteData,
	})

	indexer := Indexer{
		PageConverter: IndexPageConverter{BaseURL: pipeline.BaseURL},
		OrderedPosts:  orderer.OrderedPageSlices,
		IndexPages:    make(chan *OrderedPage[IndexPage]),
		PageSize:      10,
		Indices:       make(map[string]*Index),
	}

	indexPages := MultiChan[*OrderedPage[IndexPage]]{
		Input: indexer.IndexPages,
		Outputs: []chan *OrderedPage[IndexPage]{
			make(chan *OrderedPage[IndexPage]),
			make(chan *OrderedPage[IndexPage]),
		},
	}

	indexTemplater := NewTemplater(&TemplaterParams[IndexPage]{
		Name:        "Templater[IndexPage]",
		Concurrency: 8,
		Pages:       indexPages.Output(0),
		Output:      pipeline.OutputDirectory,
		Template:    pipeline.IndexTemplate,
		SiteData:    &pipeline.SiteData,
	})

	feedBuilder := actor.NewInput(
		"FeedBuilder",
		8,
		indexPages.Output(1),
		FeedBuilder(
			&feeds.Feed{
				Title:       "Craig Weber",
				Link:        &feeds.Link{Href: pipeline.BaseURL.String()},
				Description: "Craig Weber's blog",
				Author: &feeds.Author{
					Name:  "Craig Weber",
					Email: "weberc2@gmail.com",
				},
				Created: time.Date(2016, 1, 1, 0, 0, 0, 0, time.UTC),
			},
			pipeline.OutputDirectory,
		),
		nil,
	)

	return actor.Multi{
		&themeAssetsFinder,
		&themeAssetsCopier,
		&sourceFinder,
		&sourceReader,
		&parser,
		&orderer,
		&indexer,
		&postTemplater,
		&indexPages,
		&indexTemplater,
		&feedBuilder,
	}.Run(ctx)
}
