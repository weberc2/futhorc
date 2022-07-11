use crate::htmlrenderer::HtmlRenderer;
use crate::url::Converter as LinkConverter;
use pulldown_cmark::*;
use std::fmt;
use std::io;
use url::{ParseError as UrlParseError, Url};


/// Converts markdown to HTML, writing the result into [`w`].
///
/// * [`posts_url`] is the prefix for post URLs (e.g.,
///   https://example.org/posts/). This should end in a trailing slash.
/// * [`source_path`] is the relative path to the source file from the posts
///   directory.
/// * [`markdown`] is the contents of the source file.
/// * [`footnote_prefix`] is the prefix to prepend onto footnote links.
pub fn to_html<W: escape::StrWrite>(
    w: &mut W,
    posts_url: &Url,
    source_path: &str,
    markdown: &str,
    footnote_prefix: &str,
) -> Result<(), Error> {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_FOOTNOTES);
    options.insert(Options::ENABLE_SMART_PUNCTUATION);
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_TASKLISTS);

    let event_converter = EventConverter {
        link_converter: LinkConverter::new(posts_url, source_path)?,
    };
    let mut html_renderer =
        HtmlRenderer::with_footnote_prefix(footnote_prefix);
    for ev in Parser::new_ext(markdown, options)
        .map(|ev| event_converter.convert(ev))
    {
        let ev = ev?;
        html_renderer.on_event(w, ev)?;
    }
    Ok(())
}

struct EventConverter<'a> {
    link_converter: LinkConverter<'a>,
}

impl<'a> EventConverter<'a> {
    fn convert_tag<'b>(&self, tag: Tag<'b>) -> Result<Tag<'b>, UrlParseError> {
        Ok(match tag {
            // The headings in the post itself need to be deprecated twice to
            // be subordinate to both the site title (h1) and the post title
            // (h2). So `#` becomes h3 instead of h1. We do this by
            // intercepting heading tags and returning the tag size + 2.
            Tag::Heading(s) => Tag::Heading(s + 2),

            // Internal links (links from blog posts, pages, and assets *to*
            // posts, pages, and assets) need to be converted from their input
            // formats to their output formats (e.g., a post linking to another
            // post as `foo.md` will need to be converted to an equivalent link
            // ending in `foo.html`).
            Tag::Link(
                link @ (LinkType::Inline
                | LinkType::Reference
                | LinkType::ReferenceUnknown
                | LinkType::Shortcut
                | LinkType::Autolink
                | LinkType::Collapsed
                | LinkType::CollapsedUnknown),
                url,
                title,
            ) => Tag::Link(
                link,
                CowStr::Boxed(
                    self.link_converter.convert(&url)?.into_boxed_str(),
                ),
                title,
            ),
            _ => tag,
        })
    }

    fn convert<'b>(&self, ev: Event<'b>) -> Result<Event<'b>, UrlParseError> {
        Ok(match ev {
            Event::Start(tag) => Event::Start(self.convert_tag(tag)?),
            _ => ev,
        })
    }
}

/// Represents an error converting markdown to HTML.
#[derive(Debug)]
pub enum Error {
    /// Returned for other I/O errors.
    Io(std::io::Error),

    /// Returned when there is a problem parsing URLs.
    UrlParse(UrlParseError),
}

impl fmt::Display for Error {
    /// Displays an [`Error`] as human-readable text.
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::Io(err) => err.fmt(f),
            Error::UrlParse(err) => err.fmt(f),
        }
    }
}

impl std::error::Error for Error {
    /// Implements the [`std::error::Error`] trait for [`Error`].
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Io(err) => Some(err),
            Error::UrlParse(err) => Some(err),
        }
    }
}

impl From<url::ParseError> for Error {
    /// Converts a [`url::ParseError`] into an [`Error`]. It allows us to use
    /// the `?` operator for URL parsing and joining functions.
    fn from(err: url::ParseError) -> Error {
        Error::UrlParse(err)
    }
}

impl From<io::Error> for Error {
    /// Converts a [`io::Error`] into an [`Error`]. It allows us to use
    /// the `?` operator for IO operations.
    fn from(err: io::Error) -> Error {
        Error::Io(err)
    }
}
