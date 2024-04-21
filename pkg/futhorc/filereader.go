package futhorc

import (
	"context"
	"errors"
	"fmt"
	"futhorc/pkg/actor"
	"io"
	"io/fs"
)

type FileReader struct {
	actor actor.Map[string, FileData]
}

func NewFileReader(
	name string,
	concurrency int,
	dir fs.FS,
	sources <-chan string,
) (reader FileReader) {
	reader.actor = actor.NewMap(
		name,
		concurrency,
		sources,
		func(ctx context.Context, path string) (data FileData, err error) {
			var f fs.File
			if f, err = dir.Open(path); err != nil {
				return
			}
			defer func() { err = errors.Join(err, f.Close()) }()

			data.Path = path
			if data.Data, err = io.ReadAll(&contextReader{ctx, f}); err != nil {
				err = fmt.Errorf("reading file `%s`: %w", path, err)
			}
			return
		},
	)
	return
}

func (reader *FileReader) Output() <-chan FileData {
	return reader.actor.Output
}

func (reader *FileReader) Run(ctx context.Context) error {
	return reader.actor.Run(ctx)
}

type contextReader struct {
	ctx context.Context
	r   io.Reader
}

func (cr *contextReader) Read(p []byte) (int, error) {
	if err := cr.ctx.Err(); err != nil {
		return 0, err
	}
	return cr.r.Read(p)
}

type FileData struct {
	Path string
	Data []byte
}
