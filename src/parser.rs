//! Defines the [`Post`], [`Parser`], and [`Error`] types. Also defines the
//! logic for parsing posts from the file system into memory. See the
//! [`Post::to_value`] and [`Post::summarize`] for details on how posts are
//! converted into template values.

use std::{
    collections::HashSet,
    fmt,
    fs::{read_dir, File},
    path::{Path, PathBuf},
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

    fn parse_post_bundle(
        &self,
        posts_source_directory: &Path,
        relative_path: &Path,
        static_files: &mut Vec<StaticFile>,
    ) -> Result<Post> {
        // We want to make sure we can parse a post before we mutate
        // `static_files`
        let post = self.parse_post(
            posts_source_directory,
            &relative_path.join("index.md"),
        )?;

        // Mutate `static_files` only after we've confirmed that we've parsed a
        // valid post.
        use walkdir::WalkDir;
        let abs = posts_source_directory.join(relative_path);
        for result in WalkDir::new(&abs) {
            let entry = result?;
            if entry.file_type().is_file() && entry.file_name() != "index.md" {
                static_files.push((
                    entry.path().to_owned(),
                    self.posts_directory
                        .join(relative_path.file_name().unwrap())
                        // strip_prefix shouldn't fail since `abs` is always an
                        // ancestor of `entry_path`
                        .join(entry.path().strip_prefix(&abs).unwrap()),
                ));
            }
        }

        Ok(post)
    }

    /// Parses a single [`Post`] from an `id` and `input` strings. The `id` is
    /// the path of the file relative to the `posts_source_directory` less the
    /// extension (e.g., the ID for a post whose source file is
    /// `{posts_source_directory}/foo/bar.md` is `foo/bar`).
    fn parse_post(
        &self,
        posts_source_directory: &Path,
        relative_path: &Path,
    ) -> Result<Post> {
        match self._parse_post(posts_source_directory, relative_path) {
            Ok(p) => Ok(p),
            Err(e) => Err(Error::Annotated(
                format!("parsing post `{:?}`", relative_path),
                Box::new(e),
            )),
        }
    }

    fn _parse_post(
        &self,
        posts_source_directory: &Path,
        relative_path: &Path,
    ) -> Result<Post> {
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

        use std::io::Read;
        let mut contents = String::new();
        File::open(posts_source_directory.join(relative_path))?
            .read_to_string(&mut contents)?;
        let input: &str = &contents;

        let (yaml_start, yaml_stop, body_start) = frontmatter_indices(input)?;
        let frontmatter: Frontmatter =
            serde_yaml::from_str(&input[yaml_start..yaml_stop])?;

        let with_extension = if relative_path.ends_with("index.md") {
            relative_path.parent().unwrap()
        } else {
            relative_path
        }
        .with_extension("html");

        let file_name = with_extension
            .file_name()
            .ok_or_else(|| InvalidFileNameError(relative_path.to_owned()))?
            .to_str()
            .ok_or_else(|| InvalidFileNameError(relative_path.to_owned()))?;

        let mut post = Post {
            title: frontmatter.title,
            date: frontmatter.date,
            file_path: self.posts_directory.join(&file_name),
            url: self.posts_url.join(file_name)?,
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
            file_name,
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
    pub fn parse_posts(&self, source_directory: &Path) -> Result<Posts> {
        const MARKDOWN_EXTENSION: &str = ".md";

        let mut posts = Vec::new();
        let mut static_files = Vec::new();
        for result in read_dir(source_directory)? {
            let entry = result?;
            let os_file_name = entry.file_name();
            let file_name = os_file_name.to_string_lossy();
            if Self::is_bundle(&entry)? {
                posts.push(self.parse_post_bundle(
                    source_directory,
                    // strip_prefix() should never fail
                    entry.path().strip_prefix(source_directory).unwrap(),
                    &mut static_files,
                )?)
            } else if file_name.ends_with(MARKDOWN_EXTENSION) {
                posts.push(self.parse_post(
                    source_directory,
                    // should never fail
                    entry.path().strip_prefix(source_directory).unwrap(),
                )?);
            }
        }

        posts.sort_by(|a, b| b.date.cmp(&a.date));
        Ok((posts, static_files))
    }

    fn is_bundle(entry: &std::fs::DirEntry) -> std::io::Result<bool> {
        Ok(entry.file_type()?.is_dir()
            && entry.path().join("index.md").is_file())
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

#[derive(Debug)]
pub struct InvalidFileNameError(PathBuf);

impl fmt::Display for InvalidFileNameError {
    /// Displays an [`InvalidFileNameError`] as human-readable text.
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "invalid file name: {:?}", &self.0)
    }
}

impl std::error::Error for InvalidFileNameError {
    /// Implements the [`std::error::Error`] trait for [`InvalidFileNameError`].
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        None
    }
}

pub type Posts = (Vec<Post>, Vec<StaticFile>);

pub type StaticFile = (PathBuf, PathBuf);

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

    /// Returned for WalkDir I/O errors.
    WalkDir(walkdir::Error),

    /// Returned when a source file isn't valid UTF-8.
    InvalidFileName(InvalidFileNameError),

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
            Error::WalkDir(err) => err.fmt(f),
            Error::InvalidFileName(err) => err.fmt(f),
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
            Error::WalkDir(err) => Some(err),
            Error::InvalidFileName(err) => Some(err),
            Error::Annotated(_, err) => Some(err),
        }
    }
}

impl From<InvalidFileNameError> for Error {
    fn from(err: InvalidFileNameError) -> Error {
        Error::InvalidFileName(err)
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

impl From<walkdir::Error> for Error {
    /// Converts a [`walkdir::Error`] into an [`Error`]. It allows us to
    // use the `?` operator for fallible I/O functions.
    fn from(err: walkdir::Error) -> Error {
        Error::WalkDir(err)
    }
}

impl From<std::io::Error> for Error {
    /// Converts a [`std::io::Error`] into an [`Error`]. It allows us to
    // use the `?` operator for fallible I/O functions.
    fn from(err: std::io::Error) -> Error {
        Error::Io(err)
    }
}

#[cfg(test)]
mod test {
    use std::path::PathBuf;

    use super::*;

    #[test]
    fn test_parse_posts() -> Result<()> {
        let index_url = Url::parse("https://example.com")?;
        let posts_url = Url::parse("https://example.com/posts/")?;
        let posts_directory = Path::new("./testdata/posts/");
        let parser = Parser::new(&index_url, &posts_url, &posts_directory);
        let (posts, static_files) =
            parser.parse_posts(Path::new("./testdata/posts/"))?;

        let wanted_posts = vec![Post {
            file_path: PathBuf::from("./testdata/posts/"),
            title: String::from("Simple"),
            url: Url::parse("https://example.com/posts/simple.html")?,
            date: String::from("0000-01-01"),
            body: String::from("Today is the first day of the Common Era."),
            tags: HashSet::new(),
        }];

        // assert_eq!(wanted_posts, posts);
        Ok(())
    }
}
