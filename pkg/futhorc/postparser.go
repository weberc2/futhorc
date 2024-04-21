package futhorc

import (
	"context"
	"futhorc/pkg/actor"
)

type PostParser struct {
	actor.Map[FileData, Page[Post]]
}

func NewPostParser(
	name string,
	concurrency int,
	files <-chan FileData,
	converter *PostPageConverter,
) (parser PostParser) {
	parser.Map = actor.NewMap(
		name,
		concurrency,
		files,
		func(
			ctx context.Context,
			file FileData,
		) (page Page[Post], err error) {
			if page.Content, err = ParsePost(file.Data, file.Path); err != nil {
				return
			}
			return converter.Convert(&page.Content)
		},
	)
	return
}
