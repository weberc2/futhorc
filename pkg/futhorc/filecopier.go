package futhorc

import (
	"context"
	"errors"
	"fmt"
	"io"
	"io/fs"
	"path/filepath"

	"futhorc/pkg/actor"

	"github.com/go-git/go-billy/v5"
)

func FileCopier(dst billy.Filesystem, src fs.FS, prefix string) actor.InputCallback[string] {
	return func(ctx context.Context, path string) (err error) {
		var df billy.File
		var sf fs.File
		if df, err = dst.Create(filepath.Join(prefix, path)); err != nil {
			err = fmt.Errorf(
				"copying file `%s`; creating destination file: %w",
				path,
				err,
			)
			return
		}
		defer func() { err = errors.Join(err, df.Close()) }()

		if sf, err = src.Open(path); err != nil {
			err = fmt.Errorf(
				"copying file `%s`; opening source file: %w",
				path,
				err,
			)
			return
		}
		defer func() { err = errors.Join(err, sf.Close()) }()

		if _, err = io.Copy(df, &contextReader{ctx, sf}); err != nil {
			err = fmt.Errorf("copying file `%s`: %w", path, err)
			return
		}

		return
	}
}
