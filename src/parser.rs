//! Defines the [`Post`], [`Parser`], and [`Error`] types. Also defines the
//! logic for parsing posts from the file system into memory. See the
//! [`Post::to_value`] and [`Post::summarize`] for details on how posts are
//! converted into template values.

use std::{
    collections::HashSet,
    fmt,
    fs::{read_dir, File},
    path::Path,
};

use serde::Deserialize;
use url::Url;

use crate::{markdown, post::Post, tag::Tag};

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
                            // NOTE: tried
                            // `index_url.join(t).join("index.html")`; however,
                            // since `t` doesn't have a trailing slash,
                            // [`Url::join`] was treating it as equivalent to
                            // `index_url.join("index.html")` per the
                            // `Url::join` docs:
                            //
                            // > Note: a trailing slash is significant. Without
                            // it, the last path component is considered to be
                            // a “file” name to be removed to get at the
                            // “directory” that is used as the base
                            .join(&format!("{}/index.html", t))
                            .unwrap(), // should always succeed
                    })
                })
                .collect::<Result<HashSet<Tag>>>()?,
            body: String::default(),
        };

        markdown::to_html(
            &mut post.body,
            self.posts_url,
            id,
            &input[body_start..],
            post.url.as_str(),
        )?;
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

impl From<markdown::Error> for Error {
    fn from(err: markdown::Error) -> Error {
        match err {
            markdown::Error::Io(e) => Error::Io(e),
            markdown::Error::UrlParse(e) => Error::UrlParse(e),
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
