//! Defines the [`Post`], [`Parser`], and [`Error`] types. Also defines the
//! logic for parsing posts from the file system into memory. See the
//! [`Post::to_value`] and [`Post::summarize`] for details on how posts are
//! converted into template values.

use crate::htmlrenderer::*;
use crate::tag::Tag;
use gtmpl::Value;
use pulldown_cmark::{self, *};
use serde::Deserialize;
use std::collections::HashSet;
use std::fmt;
use std::fs::{read_dir, File};
use std::path::{Path, PathBuf};
use url::Url;

#[derive(Deserialize, Clone)]
struct Frontmatter {
    /// The title of the post.
    #[serde(rename = "Title")]
    pub title: String,

    /// The date of the post.
    #[serde(rename = "Date")]
    pub date: String,

    /// The tags associated with the post.
    #[serde(default, rename = "Tags")]
    pub tags: HashSet<String>,
}

/// Represents a blog post.
#[derive(Clone)]
pub struct Post {
    /// The output path where the final post file will be rendered.
    pub file_path: PathBuf,

    /// The address for the rendered post.
    pub url: Url,

    /// The title of the post.
    pub title: String,

    /// The date of the post.
    pub date: String,

    /// The body of the post.
    pub body: String,

    /// The tags associated with the post.
    pub tags: HashSet<Tag>,
}

impl Post {
    /// Converts a [`Post`] into a template-renderable [`Value`], representing
    /// a full post (as opposed to [`Post::summarize`] which represents a
    /// post summary). The resulting [`Value`] has fields:
    ///
    /// * `url`: The url of the post
    /// * `title`: The title of the post
    /// * `date`: The published date of the post
    /// * `body`: The post body
    /// * `tags`: A list of tags associated with the post
    pub fn to_value(&self) -> Value {
        use std::collections::HashMap;
        let mut m = HashMap::new();
        m.insert("url".to_owned(), Value::String(self.url.to_string()));
        m.insert("title".to_owned(), Value::String(self.title.clone()));
        m.insert("date".to_owned(), Value::String(self.date.clone()));
        m.insert("body".to_owned(), Value::String(self.body.clone()));
        m.insert(
            "tags".to_owned(),
            Value::Array(self.tags.iter().map(Value::from).collect()),
        );
        Value::Object(m)
    }

    /// Returns the full post body unless a `<!-- more -->` tag was found, in
    /// which case it returns the text up to that tag (the "summary" text). It
    /// also returns a boolean value indicating whether or not the tag was
    /// found.
    pub fn summary(&self) -> (&str, bool) {
        match self.body.find("<!-- more -->") {
            None => (self.body.as_str(), false),
            Some(idx) => (&self.body[..idx], true),
        }
    }

    /// Converts a [`Post`] into a template-renderable [`Value`] representing a
    /// post summary. The resulting [`Value`] has fields:
    ///
    /// * `url`: The url of the post
    /// * `title`: The title of the post
    /// * `date`: The published date of the post
    /// * `summary`: The post summary if there is a `<!-- more -->` tag or else
    ///   the full post body
    /// * `summarized`: A boolean value representing whether or not a `<!--
    ///   more -->` tag was found and thus the post was truncated.
    /// * `tags`: A list of tags associated with the post
    pub fn summarize(&self) -> Value {
        use std::collections::HashMap;
        let (summary, summarized) = self.summary();

        let mut m = HashMap::new();
        m.insert("url".to_owned(), Value::String(self.url.to_string()));
        m.insert("title".to_owned(), Value::String(self.title.clone()));
        m.insert("date".to_owned(), Value::String(self.date.clone()));
        m.insert("summary".to_owned(), Value::String(summary.to_string()));
        m.insert("summarized".to_owned(), Value::Bool(summarized));
        m.insert(
            "tags".to_owned(),
            Value::Array(self.tags.iter().map(Value::from).collect()),
        );
        Value::Object(m)
    }
}

/// Parses [`Post`] objects from source files.
pub struct Parser<'a> {
    /// `index_url` is the base URL for index pages. It's used to prefix tag
    /// page URLs (i.e., the URL for the first page of a tag is
    /// `{index_url}/{tag_name}/index.html`).
    index_url: &'a Url,

    /// `posts_url` is the base URL for post pages. It's used to prefix post
    /// page URLs (i.e., the URL for a post is
    /// `{posts_url}/{post_id}.html`).
    posts_url: &'a Url,

    /// `posts_directory` is the directory in which post pages will be
    /// rendered.
    posts_directory: &'a Path,
}

impl<'a> Parser<'a> {
    /// Constructs a new parser. See fields on [`Parser`] for argument
    /// descriptions.
    pub fn new(
        index_url: &'a Url,
        posts_url: &'a Url,
        posts_directory: &'a Path,
    ) -> Parser<'a> {
        Parser {
            index_url,
            posts_url,
            posts_directory,
        }
    }

    /// Parses a single [`Post`] from an `id` and `input` strings. The `id` is
    /// the path of the file relative to the `posts_source_directory` less the
    /// extension (e.g., the ID for a post whose source file is
    /// `{posts_source_directory}/foo/bar.md` is `foo/bar`).
    fn parse_post(&self, id: &str, input: &str) -> Result<Post> {
        match self._parse_post(id, input) {
            Ok(p) => Ok(p),
            Err(e) => Err(Error::Annotated(
                format!("parsing post `{}`", id),
                Box::new(e),
            )),
        }
    }

    fn _parse_post(&self, id: &str, input: &str) -> Result<Post> {
        fn frontmatter_indices(input: &str) -> Result<(usize, usize, usize)> {
            const FENCE: &str = "---";
            if !input.starts_with(FENCE) {
                return Err(Error::FrontmatterMissingStartFence);
            }
            match input[FENCE.len()..].find("---") {
                None => Err(Error::FrontmatterMissingEndFence),
                Some(offset) => Ok((
                    FENCE.len(),                        // yaml_start
                    FENCE.len() + offset,               // yaml_stop
                    FENCE.len() + offset + FENCE.len(), // body_start
                )),
            }
        }

        let (yaml_start, yaml_stop, body_start) = frontmatter_indices(input)?;
        let frontmatter: Frontmatter =
            serde_yaml::from_str(&input[yaml_start..yaml_stop])?;
        let file_name = format!("{}.html", id);
        let mut post = Post {
            title: frontmatter.title,
            date: frontmatter.date,
            file_path: self.posts_directory.join(&file_name),
            url: self.posts_url.join(&file_name)?,
            tags: frontmatter
                .tags
                .iter()
                .map(|t| {
                    Ok(Tag {
                        name: t.clone(),
                        url: self
                            .index_url
                            .join(t)?
                            .join("index.html")
                            .unwrap(),
                    })
                })
                .collect::<Result<HashSet<Tag>>>()?,
            body: String::default(),
        };
        let mut options = Options::empty();
        options.insert(Options::ENABLE_FOOTNOTES);
        options.insert(Options::ENABLE_SMART_PUNCTUATION);
        options.insert(Options::ENABLE_STRIKETHROUGH);
        options.insert(Options::ENABLE_TABLES);
        options.insert(Options::ENABLE_TASKLISTS);
        let parser =
            pulldown_cmark::Parser::new_ext(&input[body_start..], options);

        // The headings in the post itself need to be deprecated twice to be
        // subordinate to both the site title (h1) and the post title (h2). So
        // `#` becomes h3 instead of h1. We do this by intercepting heading
        // tags and returning the tag size + 2.
        let fixed_subheading_sizes = parser.map(|ev| match ev {
            Event::Start(tag) => Event::Start(match tag {
                pulldown_cmark::Tag::Heading(s) => {
                    pulldown_cmark::Tag::Heading(s + 2)
                }
                _ => tag,
            }),
            _ => ev,
        });

        push_html(&mut post.body, fixed_subheading_sizes, post.url.as_str())?;
        Ok(post)
    }

    /// Searches a provided `source_directory` for post files (extension =
    /// `.md`) and returns a list of [`Post`] objects sorted by date (most
    /// recent first). Each post file must be structured as follows:
    ///
    /// 1. Initial frontmatter fence (`---`)
    /// 2. YAML frontmatter with fields `Title`, `Date`, and optionally `Tags`
    /// 3. Terminal frontmatter fence (`---`)
    /// 4. Post body
    ///
    /// For example:
    ///
    /// ```md
    /// ---
    /// Title: Hello, world!
    /// Date: 2021-04-16
    /// Tags: [greet]
    /// ---
    /// # Hello
    ///
    /// World
    /// ```
    pub fn parse_posts(&self, source_directory: &Path) -> Result<Vec<Post>> {
        use std::io::Read;
        const MARKDOWN_EXTENSION: &str = ".md";

        let mut posts = Vec::new();
        for result in read_dir(source_directory)? {
            let entry = result?;
            let os_file_name = entry.file_name();
            let file_name = os_file_name.to_string_lossy();
            if file_name.ends_with(MARKDOWN_EXTENSION) {
                let base_name = file_name.trim_end_matches(MARKDOWN_EXTENSION);
                let mut contents = String::new();
                File::open(entry.path())?.read_to_string(&mut contents)?;
                posts.push(self.parse_post(base_name, &contents)?);
            }
        }

        posts.sort_by(|a, b| b.date.cmp(&a.date));
        Ok(posts)
    }
}

/// Represents the result of a [`Post`]-parse operation.
pub type Result<T> = std::result::Result<T, Error>;

/// Represents an error parsing a [`Post`] object.
#[derive(Debug)]
pub enum Error {
    /// Returned when a post source file is missing its starting frontmatter
    /// fence (`---`).
    FrontmatterMissingStartFence,

    /// Returned when a post source file is missing its terminal frontmatter
    /// fence (`---` i.e., the starting fence was found but the ending one was
    /// missing).
    FrontmatterMissingEndFence,

    /// Returned when there was an error parsing the frontmatter as YAML.
    DeserializeYaml(serde_yaml::Error),

    /// Returned when there is a problem parsing URLs.
    UrlParse(url::ParseError),

    /// Returned for other I/O errors.
    Io(std::io::Error),

    /// An error with an annotation.
    Annotated(String, Box<Error>),
}

impl fmt::Display for Error {
    /// Displays an [`Error`] as human-readable text.
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::FrontmatterMissingStartFence => {
                write!(f, "Post must begin with `---`")
            }
            Error::FrontmatterMissingEndFence => {
                write!(f, "Missing clossing `---`")
            }
            Error::DeserializeYaml(err) => err.fmt(f),
            Error::UrlParse(err) => err.fmt(f),
            Error::Io(err) => err.fmt(f),
            Error::Annotated(annotation, err) => {
                write!(f, "{}: {}", &annotation, err)
            }
        }
    }
}

impl std::error::Error for Error {
    /// Implements the [`std::error::Error`] trait for [`Error`].
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::FrontmatterMissingStartFence => None,
            Error::FrontmatterMissingEndFence => None,
            Error::DeserializeYaml(err) => Some(err),
            Error::UrlParse(err) => Some(err),
            Error::Io(err) => Some(err),
            Error::Annotated(_, err) => Some(err),
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

impl From<serde_yaml::Error> for Error {
    /// Converts a [`serde_yaml::Error`] into an [`Error`]. It allows us to use
    /// the `?` operator for [`serde_yaml`] deserialization functions.
    fn from(err: serde_yaml::Error) -> Error {
        Error::DeserializeYaml(err)
    }
}

impl From<std::io::Error> for Error {
    /// Converts a [`std::io::Error`] into an [`Error`]. It allows us to
    // use the `?` operator for fallible I/O functions.
    fn from(err: std::io::Error) -> Error {
        Error::Io(err)
    }
}
