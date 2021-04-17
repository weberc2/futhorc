use crate::htmlrenderer::*;
use crate::tag::Tag;
use crate::url::*;
use gtmpl::Value;
use pulldown_cmark::{self, *};
use serde::Deserialize;
use std::collections::HashSet;
use std::fmt;
use std::fs::{read_dir, File};
use std::path::{Path, PathBuf};

#[derive(Deserialize, Clone)]
pub struct Post {
    #[serde(default)]
    pub file_path: PathBuf,

    #[serde(default)]
    pub url: UrlBuf,

    #[serde(rename = "Title")]
    pub title: String,

    #[serde(rename = "Date")]
    pub date: String,

    #[serde(default)]
    pub body: String,

    #[serde(default, rename = "Tags")]
    pub tags: HashSet<Tag>,
}

impl Post {
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

    pub fn summarize(&self) -> Value {
        use std::collections::HashMap;
        let (summary, summarized) = match self.body.find("<!-- more -->") {
            None => (self.body.as_str(), false),
            Some(idx) => (&self.body[..idx], true),
        };

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

pub struct Parser<'a> {
    index_url: &'a Url,
    posts_url: &'a Url,
    posts_directory: &'a Path,
}

impl<'a> Parser<'a> {
    pub fn new(index_url: &'a Url, posts_url: &'a Url, posts_directory: &'a Path) -> Parser<'a> {
        Parser {
            index_url,
            posts_url,
            posts_directory,
        }
    }

    fn parse_post(&self, id: &str, input: &str) -> Result<Post> {
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
        let mut post: Post = serde_yaml::from_str(&input[yaml_start..yaml_stop])?;
        let file_name = format!("{}.html", id);
        post.url = self.posts_url.join(&file_name);
        post.file_path = self.posts_directory.join(&file_name);
        post.tags = post
            .tags
            .iter()
            .map(|t| Tag {
                tag: t.tag.clone(),
                url: self.index_url.join(&t.tag).join("index.html"),
            })
            .collect();
        let mut options = Options::empty();
        options.insert(Options::ENABLE_FOOTNOTES);
        options.insert(Options::ENABLE_SMART_PUNCTUATION);
        options.insert(Options::ENABLE_STRIKETHROUGH);
        options.insert(Options::ENABLE_TABLES);
        options.insert(Options::ENABLE_TASKLISTS);
        let parser = pulldown_cmark::Parser::new_ext(&input[body_start..], options);

        // The headings in the post itself need to be deprecated twice to be
        // subordinate to both the site title (h1) and the post title (h2). So
        // `#` becomes h3 instead of h1. We do this by intercepting heading
        // tags and returning the tag size + 2.
        let fixed_subheading_sizes = parser.map(|ev| match ev {
            Event::Start(tag) => Event::Start(match tag {
                pulldown_cmark::Tag::Heading(s) => pulldown_cmark::Tag::Heading(s + 2),
                _ => tag,
            }),
            _ => ev,
        });

        push_html(&mut post.body, fixed_subheading_sizes, post.url.as_str())?;
        Ok(post)
    }

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

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    FrontmatterMissingStartFence,
    FrontmatterMissingEndFence,
    DeserializeYaml(serde_yaml::Error),
    Io(std::io::Error),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::FrontmatterMissingStartFence => write!(f, "Post must begin with `---`"),
            Error::FrontmatterMissingEndFence => write!(f, "Missing clossing `---`"),
            Error::DeserializeYaml(err) => err.fmt(f),
            Error::Io(err) => err.fmt(f),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::FrontmatterMissingStartFence => None,
            Error::FrontmatterMissingEndFence => None,
            Error::DeserializeYaml(err) => Some(err),
            Error::Io(err) => Some(err),
        }
    }
}

impl From<serde_yaml::Error> for Error {
    fn from(err: serde_yaml::Error) -> Error {
        Error::DeserializeYaml(err)
    }
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Error {
        Error::Io(err)
    }
}
