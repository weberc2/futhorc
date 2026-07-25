// Package futhorc is the static-site generator: it parses Markdown posts,
// renders them (and index/tag pages) through a themed template pipeline, and
// writes the resulting site to disk.
package futhorc

type Site struct {
	HomePage  string
	AtomURL   string
	StaticURL string
}
