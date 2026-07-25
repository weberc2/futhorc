package futhorc

import (
	"fmt"
	"net/url"
	"path/filepath"
)

type IndexPageConverter PageConverter[IndexPage]

func (converter *IndexPageConverter) Convert(
	idx *Index,
	pageNumber int,
	postsStart int,
	postsEnd int,
) (page Page[IndexPage], err error) {
	var fileName string
	if pageNumber == 0 {
		fileName = "index.html"
	} else {
		fileName = fmt.Sprintf("page-%03d.html", pageNumber)
	}
	return (*PageConverter[IndexPage])(converter).Convert(
		filepath.Join(idx.ID, fileName),
		int64(pageNumber),
		IndexPage{
			IndexID: idx.ID,
			Number:  pageNumber,
			Posts:   idx.Posts[postsStart:postsEnd],
		},
	)
}

type IndexPage struct {
	IndexID string
	Number  int
	Posts   []*OrderedPage[Post]
}

func (page *IndexPage) PageContent(
	base *url.URL,
) (c Page[IndexPage], err error) {
	c = Page[IndexPage]{
		Content: *page,
		Order:   int64(page.Number),
		Path: filepath.Join(
			page.IndexID,
			fmt.Sprintf("page-%03d.html", page.Number),
		),
	}
	var rel *url.URL
	if rel, err = url.Parse(c.Path); err != nil {
		err = fmt.Errorf(
			"creating url for index page `%d`: %w",
			page.Number,
			err,
		)
		return
	}

	c.URL = base.ResolveReference(rel)
	return
}
