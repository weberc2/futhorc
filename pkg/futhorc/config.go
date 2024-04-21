package futhorc

import (
	"encoding/json"
	"fmt"
	"html/template"
	"net/url"
	"os"
	"path/filepath"

	"github.com/go-git/go-billy/v5/osfs"
	"github.com/tailscale/hujson"
)

func LoadPipeline(dir string) (pipeline Pipeline, err error) {
	var config Config
	if config, err = LoadConfig(dir); err != nil {
		err = fmt.Errorf("loading pipeline: %w", err)
		return
	}

	if pipeline, err = config.Pipeline(); err != nil {
		err = fmt.Errorf("loading pipeline: %w", err)
		return
	}

	return
}

type Config struct {
	SiteRootURL     string `json:"siteRootURL"`
	InputDirectory  string `json:"-"`
	OutputDirectory string `json:"-"`
}

func LoadConfig(dir string) (config Config, err error) {
	var data []byte
	if data, err = os.ReadFile(
		filepath.Join(dir, "futhorc.jsonc"),
	); err != nil {
		err = fmt.Errorf("loading config from `futhorc.jsonc`: %w", err)
		return
	}
	if data, err = hujson.Standardize(data); err != nil {
		err = fmt.Errorf("standardizing input from `futhorc.jsonc`: %w", err)
		return
	}
	config.InputDirectory = dir
	if err = json.Unmarshal(data, &config); err != nil {
		err = fmt.Errorf("unmarshaling `futhorc.jsonc`: %w", err)
		return
	}
	return
}

func (c *Config) Pipeline() (pipeline Pipeline, err error) {
	if pipeline.BaseURL, err = url.Parse(c.SiteRootURL); err != nil {
		return
	}

	if c.OutputDirectory == "" {
		c.OutputDirectory = filepath.Join(c.InputDirectory, "_output")
	}

	pipeline.PostSources = os.DirFS(filepath.Join(c.InputDirectory, "posts"))
	pipeline.ThemeAssets = os.DirFS(filepath.Join(
		c.InputDirectory,
		"theme/static",
	))
	pipeline.OutputDirectory = osfs.New(filepath.Join(c.OutputDirectory))
	pipeline.OutputDirectoryThemeAssets = osfs.New(filepath.Join(
		c.OutputDirectory,
		"static",
		"theme",
	))

	var theme Theme
	if theme, err = LoadTheme(os.DirFS(filepath.Join(
		c.InputDirectory,
		"theme",
	))); err != nil {
		return
	}

	pipeline.PostTemplate = theme.PostTemplate
	pipeline.IndexTemplate = theme.IndexTemplate

	pipeline.SiteData = SiteData{
		BaseURL: template.URL(pipeline.BaseURL.String()),
		HomePage: template.URL(
			pipeline.BaseURL.JoinPath("index.html").String(),
		),
		ThemeAssets: template.URL(
			pipeline.BaseURL.JoinPath("static/theme/").String(),
		),
		FeedURL: template.URL(
			pipeline.BaseURL.JoinPath("index.json").String(),
		),
		FeedType: "application/json",
	}

	return
}
