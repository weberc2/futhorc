package futhorc

import (
	"context"
	"futhorc/pkg/actor"
	"io/fs"
	"path/filepath"
	"strings"
)

func FileFinder(root fs.FS, extension string) actor.OutputCallback[string] {
	dirs := []string{"."}
	var dir string
	var entries []fs.DirEntry
	return func(ctx context.Context) (string, error) {
		// read until we find an entry that matches the extension
		for {
			// scan any existing entries (if `entries` is non-empty, it
			// means we previously scanned a directory and found a matching
			// entry--now we're scanning the rest of the entries from that
			// directory to see if there are more matches).
			var entry fs.DirEntry
			for len(entries) > 0 {
				entry, entries = entries[0], entries[1:]
				path := filepath.Join(dir, entry.Name())
				if entry.IsDir() {
					dirs = append(dirs, path)
				} else if strings.HasSuffix(entry.Name(), extension) {
					return path, nil
				}
			}

			// if we get here, then entries is empty and we should try to
			// refill it by reading the next directory on the `dirs` stack.
			if len(dirs) < 1 {
				return "", actor.ErrStop
			}
			dir, dirs = dirs[0], dirs[1:]

			// before doing any i/o, let's check the context...
			if err := ctx.Err(); err != nil {
				return "", err
			}

			var err error
			if entries, err = fs.ReadDir(root, dir); err != nil {
				return "", err
			}
		}
	}
}
