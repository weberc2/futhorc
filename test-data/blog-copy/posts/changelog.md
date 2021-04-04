---
Title: Changelog
Date: 2021-01-16
Tags: [homelab]
---

I worked on this blog for several hours this weekend, releasing a new post,
implementing a couple new features and fixing some bugs. Here are the
highlights:

* Published the [first entry][0] in my [Homelab][1] series
* Fixed a bug on iPad that was causing a ~300% zoom
* Implemented syndication (atom feed)
* Fixed broken relative links in post snippets
* Reduced coupling between markdown and site configuration

<!-- more -->

# iPad Viewport Bug

Jen got a new iPad this morning, so I decided to check out how this blog looked
in iPad format. It turns out, not great. Everything was zoomed in about 3x. It
looks good on desktop and on mobile, but for some reason, on iPad everything is
magnified.

I don't understand exactly why it only affects the iPad, but the root cause is
that, while I was configuring the HTML and CSS to support mobile devices, I
added this tag:

```html
<meta name=viewport content="width=350">
```

I added this because without it, opening up my website on mobile would just
show a whole bunch of the left-margin, and the user would have to pan around
to see various parts of the text. I didn't know at the time why this fixed it,
and I largely still don't know; however, this was telling other browsers (at
least on iPad) to scale everything up. The fix was to set this width property
to a special `device-width` value.


# Syndication

One of the reasons I built this blog is that the idea of blogging harkens back
to the pre-social-media days when the Internet was smaller, more heterogeneous,
and decentralized. Syndication (RSS/Atom feeds) were never completely pervasive
at the time, but they also seem to fit the aesthetic of that earlier, more
decentralized, era, and I've been meaning to implement syndication since I
first built the blog.

The reasons I hadn't tackled it earlier was because I hadn't found a library
that was convenient, and I was also concerned that I would have to
significantly alter my DIY static site generator's ([Neon][2]) architecture and
I didn't really want to bite off that much while I wasn't even updating my blog
regularly.

I decided to take a stab at it this weekend. I found a delightfully simple feed
library ([gorilla/feeds][3]) and it integrated neatly into my existing
architecture. It only took me ~an hour to complete the feature. The bulk of the
work is [here][4]:

```go
func buildFeed(conf config.Config, posts ByDate) error {
	var now time.Time
	if len(posts) > 0 {
		now = time.Time(posts[0].Date)
	} else {
		now = time.Now()
	}

	feed := &feeds.Feed{
		Title:       conf.Feed.Title,
		Link:        &feeds.Link{Href: conf.SiteRoot},
		Description: conf.Feed.Description,
		Author:      &feeds.Author{Name: conf.Feed.Author},
		Created:     now,
	}
	for _, post := range posts {
		feed.Items = append(
			feed.Items,
			&feeds.Item{
				Title:       post.Title,
				Link:        &feeds.Link{Href: relLink(conf.SiteRoot, post.ID)},
				Author:      &feeds.Author{Name: conf.Feed.Author},
				Created:     time.Time(post.Date),
				Description: string(snippet(post.Body)),
			},
		)
	}

	file, err := os.Create(filepath.Join(conf.OutputDirectory, "feed.atom"))
	if err != nil {
		return err
	}
	defer func() {
		if err := file.Close(); err != nil {
			log.Printf("ERROR Failed to close file: %v", err)
		}
	}()

	return feed.WriteAtom(file)
}
```

# Markdown Table

For my [Homelab/Hardware post][0], I wanted an HTML table to represent my bill
of materials for my Raspberry Pi cluster. Neon uses a high-quality, extensible
markdown library, [blackfriday][5]. As it turns out, the library has built-in
support for markdown tables that we can enable by bitwise-OR-ing it into the
list of extensions ([source][6]):

```go
blackfriday.Run(
    // ...
    blackfriday.WithExtensions(
        blackfriday.CommonExtensions|
            blackfriday.Footnotes|
            blackfriday.Tables,
    ),
    // ...
)
```

That generates a bare HTML table, but I still had to style it with CSS to make
it render like one would expect when viewing it on a website.

# Broken links in snippets

When writing a post such as this one, I often want to link to other posts that
I've written. I don't want to hard-code the `SiteRoot` (e.g.,
example.org/blog/) in case I move the blog to another domain or move it under
a /blog/ prefix. So I would use a relative link (e.g., ./foo.html). This worked
fine for people who were reading the post from the post's page; however, it
404-ed if the link was part of the snippet text displayed on an index page
(e.g., index.html) or in a feed-reader. The reason is because my blog puts
posts under a `/posts/` prefix (which is itself below the "site root", which is
a combination of the host and an optional prefix) while the main index page is
at `/index.html` (in the site root) and other index pages are under a `/pages/`
prefix (e.g., `/pages/1.html`.

For example, given a post `foo.html` that's linking to another post,
`bar.html`, when we're viewing the `foo` post, the browser is at
`{site-root}/posts/foo.html` and the `./bar.html` relative link resolves to
`{site-root}/posts/bar.html`; however, when we're viewing the `foo` snippet on
the `{site-root}/index.html` page, the `./bar.html` link resolves to
`{site-root}/bar.html` instead of `{site-root}/posts/bar.html`

The solution had to allow for the `PostOutputDirectory` (the `/posts/` prefix)
and the `SiteRoot` to remain configurable, which meant I couldn't require
links to hard-code these values. Instead, I tweaked the markdown renderer to
replace relative links with fully-qualified, absolute links. So a link like
this: `[bar](./bar.html)` would be rendered as
`{site_root}/{post_output_dir}/bar.html` (e.g.,
`example.org/blog/posts/bar.html`).

This was pretty easy because blackfriday has a [`Renderer`][7] interface that
we can implement to customize the rendering. This interface has a method
`RenderNode(w io.Writer, node *Node, entering bool) WalkStatus`, which is
invoked on each node in the parse tree. To implement the link replacement,
I'm implementing my own renderer that wraps some base renderer. When
`RenderNode()` is invoked on anything besides relative link nodes, the
customrenderer immediately delegates to the base renderer's `RenderNode()`. If
it *is* a relative link node, then the custom renderer will create a new
absolute link node and pass that into the base renderer ([source][8]):

```go
type renderer struct {
	blackfriday.Renderer
	linkPrefix      string
}

func (r *renderer) RenderNode(
	w io.Writer,
	node *blackfriday.Node,
	entering bool,
) blackfriday.WalkStatus {
	prefix := []byte("./")
	n := *node // copy the node

    // if the node is a relative link, then make it an absolute link
	if bytes.HasPrefix(n.LinkData.Destination, prefix) {
		n.LinkData.Destination = []byte(fmt.Sprintf(
			"%s/%s",
			r.linkPrefix,
			n.LinkData.Destination[len(prefix):],
		))
	}

    // call the base renderer with the copied, potentially absolute-link, node
	return r.Renderer.RenderNode(w, &n, entering)
}
```

# Decoupling source files from Neon details

Part of my philosophy for Neon is that the input markdown files should be
loosely-coupled from various details about Neon and from its configuration. If
something changes in the configuration or in Neon itself, I shouldn't have to
go back and update a bunch of markdown files. One deviation from that
philosophy was that links between posts had to be expressed in the source file
as a link to the target post's output file. In other words, the link had to
know the filename and extension of the output file.

While I was dabbling with the renderer, I decided to also allow for expressing
links to other posts' source files. This amounted to a one-line find/replace
(`s/.md/.html`) on the link node's `Destination` field, bringing Neon more
in-line with its own philosophy.


[0]: ./homelab-part-i-hardware.md
[1]: ./homelab-intro.md
[2]: https://github.com/weberc2/neon
[3]: https://pkg.go.dev/github.com/gorilla/feeds
[4]: https://github.com/weberc2/neon/blob/9275ef8029a8325d7d1b08b011adaa6c9238b2d3/build/feed.go
[5]: https://pkg.go.dev/gopkg.in/russross/blackfriday.v2
[6]: https://github.com/weberc2/neon/blob/9275ef8029a8325d7d1b08b011adaa6c9238b2d3/build/util.go#L40-L44
[7]: https://pkg.go.dev/gopkg.in/russross/blackfriday.v2#Renderer
[8]: https://github.com/weberc2/neon/blob/9275ef8029a8325d7d1b08b011adaa6c9238b2d3/build/util.go#L64-L100
