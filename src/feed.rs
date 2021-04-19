//! Support for creating Atom feeds from a list of posts.

use crate::config::Author;
use crate::post::Post;
use crate::url::UrlBuf;
use atom_syndication::{Entry, Error as AtomError, Feed, Link, Person};
use chrono::{
    FixedOffset, NaiveDate, NaiveDateTime, NaiveTime, ParseError, ParseResult, TimeZone, Utc,
};
use std::fmt;
use std::io::Write;

/// Bundled configuration for creating a feed.
pub struct FeedConfig {
    pub title: String,
    pub id: String,
    pub author: Option<Author>,
    pub home_page: UrlBuf,
}

/// Creates a feed from some configuration ([`FeedConfig`]) and a list of
/// [`Post`]s and writes the result to a [`std::io::Write`]. This function takes
/// ownership of the provided [`FeedConfig`].
pub fn write_feed<W: Write>(config: FeedConfig, posts: &[Post], w: W) -> Result<()> {
    feed(config, posts)?.write_to(w)?;
    Ok(())
}

fn feed(config: FeedConfig, posts: &[Post]) -> ParseResult<Feed> {
    use std::collections::HashMap;
    Ok(Feed {
        entries: feed_entries(&config, posts)?,
        title: config.title,
        id: config.id,
        updated: FixedOffset::east(0).from_utc_datetime(&Utc::now().naive_utc()),
        authors: author_to_people(config.author),
        categories: Vec::new(),
        contributors: Vec::new(),
        generator: None,
        icon: None,
        logo: None,
        rights: None,
        subtitle: None,
        extensions: HashMap::new(),
        namespaces: HashMap::new(),
        links: vec![Link {
            href: config.home_page.into_string(),
            rel: "alternate".to_string(),
            title: None,
            hreflang: None,
            mime_type: None,
            length: None,
        }],
    })
}

fn feed_entries(config: &FeedConfig, posts: &[Post]) -> ParseResult<Vec<Entry>> {
    use std::collections::HashMap;
    let mut entries: Vec<Entry> = Vec::with_capacity(posts.len());

    for post in posts {
        let (summary, _) = post.summary();

        // Good grief, `chrono` is ridiculous. If we try to skip this ceremony
        // and just do FixedOffset::parse_from_str(), we will get a runtime
        // error because we don't have fully-precise time information or a
        // timezone. Below I'm intending to use the UTC timezone. I think that's
        // what `FixedOffset::east(0)` does, but it's hard to say because chrono
        // is so complicated and the documentation doesn't provide enough
        // context.
        let naive_date = NaiveDate::parse_from_str(&post.date, "%Y-%m-%d")?;
        let naive_time = NaiveTime::from_hms(0, 0, 0);
        let naive_date_time = NaiveDateTime::new(naive_date, naive_time);
        let offset = FixedOffset::east(0);
        let date = offset.from_utc_datetime(&naive_date_time);

        entries.push(Entry {
            id: post.url.to_string(),
            title: post.title.clone(),
            updated: date,
            authors: author_to_people(config.author.clone()),
            links: vec![Link {
                href: post.url.to_string(),
                rel: "alternate".to_owned(),
                title: None,
                mime_type: None,
                hreflang: None,
                length: None,
            }],
            rights: None,
            summary: Some(summary.to_owned()),
            categories: Vec::new(),
            contributors: Vec::new(),
            published: Some(date),
            source: None,
            content: None,
            extensions: HashMap::new(),
        })
    }
    Ok(entries)
}

fn author_to_people(author: Option<Author>) -> Vec<Person> {
    match author {
        Some(author) => vec![Person {
            name: author.name,
            email: author.email,
            uri: None,
        }],
        None => Vec::new(),
    }
}

type Result<T> = std::result::Result<T, Error>;

/// Represents a problem creating a feed. Variants inlude I/O, Atom, and
/// date-time parsing issues.
#[derive(Debug)]
pub enum Error {
    /// Returned when there is a generic I/O error.
    Io(std::io::Error),

    /// Returned when there is an Atom-related error.
    Atom(AtomError),

    /// Returned when there is an issue parsing a post's date.
    DateTimeParse(ParseError),
}

impl fmt::Display for Error {
    /// Implements [`fmt::Display`] for [`Error`].
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::Io(err) => err.fmt(f),
            Error::Atom(err) => err.fmt(f),
            Error::DateTimeParse(err) => err.fmt(f),
        }
    }
}

impl std::error::Error for Error {
    /// Implements [`std::error::Error`] for [`Error`].
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Io(err) => Some(err),
            Error::Atom(err) => Some(err),
            Error::DateTimeParse(err) => Some(err),
        }
    }
}

impl From<std::io::Error> for Error {
    /// Converts [`std::io::Error`]s into [`Error`]. This allows us to use the
    /// `?` operator in fallible feed operations.
    fn from(err: std::io::Error) -> Error {
        Error::Io(err)
    }
}

impl From<AtomError> for Error {
    /// Converts [`AtomError`]s into [`Error`]. This allows us to use the `?`
    /// operator in fallible feed operations.
    fn from(err: AtomError) -> Error {
        Error::Atom(err)
    }
}

impl From<ParseError> for Error {
    /// Converts [`ParseError`]s into [`Error`]. This allows us to use the `?`
    /// operator in fallible feed operations.
    fn from(err: ParseError) -> Error {
        Error::DateTimeParse(err)
    }
}
