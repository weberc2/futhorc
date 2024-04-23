package futhorc

import (
	"encoding/json"
	"fmt"
	"html/template"
	"io/fs"
	"net/url"
	"strings"

	"github.com/tailscale/hujson"
)

type Theme struct {
	IndexTemplate *template.Template
	PostTemplate  *template.Template
	Assets        fs.FS
}

func LoadTheme(dir fs.FS) (theme Theme, err error) {
	var data []byte
	if data, err = fs.ReadFile(dir, "theme.jsonc"); err != nil {
		err = fmt.Errorf("loading theme: %w", err)
		return
	}
	if data, err = hujson.Standardize(data); err != nil {
		err = fmt.Errorf("loading theme: %w", err)
		return
	}
	var spec struct {
		IndexTemplate []string `json:"indexTemplate"`
		PostTemplate  []string `json:"postTemplate"`
	}
	if err = json.Unmarshal(data, &spec); err != nil {
		err = fmt.Errorf("loading theme: %w", err)
		return
	}

	theme.IndexTemplate, err = parse(dir, spec.IndexTemplate...)
	if err != nil {
		err = fmt.Errorf("loading theme: %w", err)
		return
	}

	theme.PostTemplate, err = parse(dir, spec.PostTemplate...)
	if err != nil {
		err = fmt.Errorf("loading theme: %w", err)
		return
	}

	if theme.Assets, err = fs.Sub(dir, "static"); err != nil {
		err = fmt.Errorf("loading theme: %w", err)
		return
	}

	return
}

func parse(fs fs.FS, templates ...string) (*template.Template, error) {
	return template.New(templates[0]).
		Funcs(template.FuncMap{
			"url": func(url *url.URL) template.URL {
				return template.URL(url.String())
			},
			"html": func(input string) template.HTML {
				return template.HTML(input)
			},
			"startswith": strings.HasPrefix,
		}).
		ParseFS(fs, templates...)
}
