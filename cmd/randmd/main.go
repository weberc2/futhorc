package main

import (
	"encoding/base64"
	"fmt"
	"futhorc/pkg/futhorc"
	"html/template"
	"log"
	"math/rand"
	"os"
	"path/filepath"
	"time"
	"unsafe"

	"gopkg.in/yaml.v3"
)

func main() {
	rand.Seed(time.Now().Unix())
	if err := os.RemoveAll("./example2"); err != nil {
		if !os.IsNotExist(err) {
			log.Fatal(err)
		}
	}

	if err := randMD("./example2/posts"); err != nil {
		log.Fatal(err)
	}
}

func randMD(dir string) error {
	if err := os.MkdirAll(dir, 0766); err != nil {
		return err
	}

	for range 1_000 {
		post := randPost()
		path := filepath.Join(dir, post.Path)
		f, err := os.Create(path)
		if err != nil {
			return err
		}

		frontmatter, err := yaml.Marshal(&post.Frontmatter)
		if err != nil {
			return fmt.Errorf("marshaling frontmatter: %w", err)
		}

		if err := func() error {
			defer f.Close()

			for _, data := range [][]byte{
				[]byte("---\n"),
				frontmatter,
				[]byte("\n---\n"),
				[]byte(post.Body),
			} {
				if _, err := f.Write(data); err != nil {
					return fmt.Errorf("writing post file: %w", err)
				}
			}
			return nil
		}(); err != nil {
			return err
		}
	}

	return nil
}

func randPost() futhorc.Post {
	const (
		metadataMin = 10
		metadataMax = 512
	)
	tagNames := randStringSlice(
		0,
		[]string{"foo", "bar", "baz", "qux", "alpha", "beta", "gamma", "omega"},
	)
	tags := make([]futhorc.Link, len(tagNames))
	for i := range tagNames {
		tags[i].Text = tagNames[i]
	}
	return futhorc.Post{
		Frontmatter: futhorc.Frontmatter{
			Title:  randString(metadataMin, metadataMax),
			Author: randString(metadataMin, metadataMax),
			Date:   futhorc.Date(time.Unix(2000+rand.Int63n(60), rand.Int63())),
			Tags:   tags,
		},
		Path: randString(5, 50) + ".md",
		Body: template.HTML(randString(512, 1024*1024)),
	}
}

func randStringSlice(sliceMin int, items []string) []string {
	out := make([]string, randInt(sliceMin, len(items)))
	seen := make(map[string]struct{})
	for i := range out {
		var exists bool = true
		var item string
		for exists {
			item = randGrab(items)
			if _, exists = seen[item]; exists {
				continue
			}
		}
		seen[item] = struct{}{}
		out[i] = item
	}
	return out
}

func randGrab(items []string) string {
	return items[randInt(0, len(items))]
}

func randInt(min, max int) int {
	return min + rand.Intn(max-min)
}

func randData(min, max int) []byte {
	buf := make([]byte, randInt(min, max))
	rand.Read(buf)
	out := make([]byte, base64.RawURLEncoding.EncodedLen(len(buf)))
	base64.RawURLEncoding.Encode(out, buf)
	return out
}

func randString(min, max int) string {
	out := randData(min, max)
	base64.RawStdEncoding.EncodeToString(out)
	return *(*string)(unsafe.Pointer(&out))
}
