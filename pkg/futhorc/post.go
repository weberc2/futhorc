package futhorc

import (
	"bytes"
	"errors"
	"fmt"
	"html/template"
	"strings"
	"time"
	"unsafe"

	"gopkg.in/yaml.v3"
)

type Post struct {
	// Frontmatter is the metadata about the post parsed from frontmatter.
	Frontmatter `yaml:",inline"`

	// Path is the relative path to the file from the put posts directory.
	Path string

	// Body is the post body source.
	Body template.HTML

	// Snippet is the body text before the first `<!-- more -->` tag.
	Snippet template.HTML
}

func ParsePost(data []byte, sourcePath string) (p Post, err error) {
	var idx int
	if !bytes.HasPrefix(data, startFence) {
		err = ErrFrontmatterMissingStartFence
		goto ERROR
	}

	idx = bytes.Index(data, endFence)
	if idx < 1 {
		err = ErrFrontmatterMissingEndFence
		goto ERROR
	}

	if err = yaml.Unmarshal(
		data[len(startFence):idx],
		&p.Frontmatter,
	); err != nil {
		goto ERROR
	}

	data = data[idx+len(endFence):] // calculate the body and save it as `data`
	p.Body = *(*template.HTML)(unsafe.Pointer(&data))
	p.Path = sourcePath
	return
ERROR:
	err = fmt.Errorf("parsing post `%s`: %w", sourcePath, err)
	return
}

type Frontmatter struct {
	Title  string `yaml:"Title"`
	Author string `yaml:"Author"`
	Date   Date   `yaml:"Date"`
	Tags   []Link `yaml:"Tags"`
}

type Link struct {
	Text string
	URL  template.URL
}

func (l *Link) MarshalYAML() (interface{}, error) {
	return l.Text, nil
}

func (l *Link) UnmarshalYAML(value *yaml.Node) error {
	// we're only going to unmarshal the text in frontmatter
	err := value.Decode(&l.Text)
	l.Text = strings.ToLower(l.Text)
	return err
}

type Date time.Time

func (d Date) Before(other Date) bool {
	return time.Time(d).Before(time.Time(other))
}

func (d Date) String() string {
	return time.Time(d).Format(dateLayout)
}

func (d *Date) UnmarshalYAML(value *yaml.Node) error {
	var s string
	var err error
	if err = value.Decode(&s); err != nil {
		return fmt.Errorf("unmarshaling time: %w", err)
	}
	if *(*time.Time)(d), err = time.Parse(dateLayout, s); err != nil {
		return fmt.Errorf("unmarshaling time: %w", err)
	}
	return nil
}

func (d Date) MarshalYAML() (interface{}, error) {
	return time.Time(d).Format(dateLayout), nil
}

const dateLayout = "2006-01-02"

var (
	ErrFrontmatterMissingStartFence = errors.New(
		"scanning frontmatter: missing start fence",
	)
	ErrFrontmatterMissingEndFence = errors.New(
		"scanning frontmatter: missing end fence",
	)

	blockSize  = 1024
	startFence = []byte(startFenceString)
	endFence   = []byte(endFenceString)
)

const startFenceString = "---\n"
const endFenceString = "\n---\n"
