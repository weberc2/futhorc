//! Defines the [`Post`], [`Parser`], and [`Error`] types. Also defines the
//! logic for parsing posts from the file system into memory. See the
//! [`Post::to_value`] and [`Post::summarize`] for details on how posts are
//! converted into template values.

use crate::htmlrenderer::*;
use crate::normalize_url;
use crate::tag::Tag;
use crate::url::UrlBuf;
use gtmpl::Value;
use pulldown_cmark::{self, *};
use serde::Deserialize;
use std::collections::HashSet;
use std::fmt;
use std::fs::{read_dir, File};
use std::path::{Path, PathBuf};

#[derive(Default, Deserialize, Clone)]
pub struct StaticFile {
    /// The path to the source data.
    #[serde(default)]
    pub source: PathBuf,

    /// The output path where the final post file will be rendered.
    #[serde(default)]
    pub destination: PathBuf,
}

/// Represents a blog post.
#[derive(Deserialize, Clone)]
pub struct Post {
    /// The output path where the final post file will be rendered.
    #[serde(default)]
    pub file_path: PathBuf,

    /// The address for the rendered post.
    #[serde(default)]
    pub url: UrlBuf,

    /// The title of the post.
    #[serde(rename = "Title")]
    pub title: String,

    /// The date of the post.
    #[serde(rename = "Date")]
    pub date: String,

    /// The body of the post.
    #[serde(default)]
    pub body: String,

    /// The tags associated with the post.
    #[serde(default, rename = "Tags")]
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
    index_url: &'a url::Url,

    /// `posts_url` is the base URL for post pages. It's used to prefix post
    /// page URLs (i.e., the URL for a post is
    /// `{posts_url}/{post_id}.html`).
    posts_url: &'a url::Url,

    /// `posts_directory` is the directory in which post pages will be
    /// rendered.
    posts_directory: &'a Path,
}

impl<'a> Parser<'a> {
    /// Constructs a new parser. See fields on [`Parser`] for argument
    /// descriptions.
    pub fn new(
        index_url: &'a url::Url,
        posts_url: &'a url::Url,
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
    fn parse_post(&self, post_path: &str, id: &str, file: &mut File) -> Result<Post> {
        use std::io::Read;

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

        let mut contents = String::new();
        file.read_to_string(&mut contents)?;

        let (yaml_start, yaml_stop, body_start) =
            frontmatter_indices(&contents)?;
        let mut post: Post =
            serde_yaml::from_str(&contents[yaml_start..yaml_stop])?;
        let file_name = format!("{}.html", id);
        post.url = UrlBuf::from(self.posts_url.join(&file_name)?.as_str());
        post.file_path = self.posts_directory.join(&file_name);
        post.tags = post
            .tags
            .iter()
            .map(|t| Tag {
                name: t.name.clone(),
                url: UrlBuf::from(
                    self.index_url
                        .join(&t.name)
                        .unwrap()
                        .join("index.html")
                        .unwrap()
                        .to_string(),
                ),
            })
            .collect();
        let mut options = Options::empty();
        options.insert(Options::ENABLE_FOOTNOTES);
        options.insert(Options::ENABLE_SMART_PUNCTUATION);
        options.insert(Options::ENABLE_STRIKETHROUGH);
        options.insert(Options::ENABLE_TABLES);
        options.insert(Options::ENABLE_TASKLISTS);

        //TODO: look into storing self.posts_url as a `url::Url`
        let posts_url = url::Url::parse(self.posts_url.as_str())?;
        let events =
            pulldown_cmark::Parser::new_ext(&contents[body_start..], options)
                // The headings in the post itself need to be deprecated twice to
                // be subordinate to both the site title (h1) and the post title
                // (h2). So `#` becomes h3 instead of h1. We do this by
                // intercepting heading tags and returning the tag size + 2.
                .map(|ev| match ev {
                    Event::Start(tag) => Event::Start(match tag {
                        pulldown_cmark::Tag::Heading(s) => {
                            pulldown_cmark::Tag::Heading(s + 2)
                        }
                        _ => tag,
                    }),
                    _ => ev,
                })
                .map(|ev| {
                    Result::<Event>::Ok(match ev {
                        Event::Start(tag) => Event::Start(match tag {
                            pulldown_cmark::Tag::Link(
                                LinkType::Inline,
                                url,
                                title,
                            ) => pulldown_cmark::Tag::Link(
                                LinkType::Inline,
                                CowStr::Boxed(
                                    normalize_url::convert(
                                        &posts_url,
                                        post_path,
                                        &url,
                                    )?
                                    .into_boxed_str(),
                                ),
                                title,
                            ),
                            _ => tag,
                        }),
                        _ => ev,
                    })
                });

        let mut renderer = HtmlRenderer::new();
        for ev in events {
            renderer.on_event(&mut post.body, ev?)?;
        }
        Ok(post)
    }

    fn parse_post_directory(
        &self,
        id: &str,
        dir: &Path,
        static_files: &mut Vec<StaticFile>,
    ) -> Result<Post> {
        let posts_relative_path: PathBuf = PathBuf::from(dir.file_name().unwrap()).join("index.md");
        let post = self.parse_post(
            &posts_relative_path.to_string_lossy(),
            id,
            &mut File::open(&dir.join("index.md"))?,
        )?;
        for result in read_dir(dir)? {
            let entry = result?;
            let file_name = entry.file_name();
            if file_name != "index.md" {
                static_files.push(StaticFile {
                    source: entry.path(),
                    destination: self
                        .posts_directory
                        .join(id)
                        .join(&file_name),
                });
            }
        }
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
    pub fn parse_posts(
        &self,
        source_directory: &Path,
    ) -> Result<(Vec<Post>, Vec<StaticFile>)> {
        let mut posts = Vec::new();
        let mut static_files = Vec::new();
        for result in read_dir(source_directory)? {
            let entry = result?;
            let os_file_name = entry.file_name();
            let file_name = os_file_name.to_string_lossy();
            let id = file_name.trim_end_matches(MARKDOWN_EXTENSION);

            if file_name.ends_with(MARKDOWN_EXTENSION) {
                posts.push(
                    self.parse_post(
                        &file_name,
                        id,
                        &mut File::open(entry.path())?,
                    )?,
                );
            } else if entry.file_type()?.is_dir() {
                // if the entry is a directory containing an index.md file,
                // parse a post from the directory
                if entry.path().join("index.md").is_file() {
                    posts.push(self.parse_post_directory(
                        id,
                        &entry.path(),
                        &mut static_files,
                    )?);
                }
            }
        }

        posts.sort_by(|a, b| b.date.cmp(&a.date));
        Ok((posts, static_files))
    }
}

pub const MARKDOWN_EXTENSION: &str = ".md";
pub const HTML_EXTENSION: &str = ".html";

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

    /// Returned when there was an error parsing or joining URLs.
    UrlParse(url::ParseError),

    /// Returned for other I/O errors.
    Io(std::io::Error),
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
        }
    }
}

impl From<url::ParseError> for Error {
    /// Converts a [`url::ParseError`] into an [`Error`]. It allows us to use
    /// the `?` operator for [`url`] parse and join functions.
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
