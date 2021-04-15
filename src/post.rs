use crate::htmlrenderer::push_html;
use crate::url::{Url, UrlBuf};
use pulldown_cmark::{self, Event, Options, Parser};
use serde::{Deserialize, Deserializer};
use serde_yaml;
use std::fmt::{self, Display, Formatter};
use std::fs::{read_dir, File};
use std::path::Path;

#[derive(Clone, Debug)]
pub struct Tag {
    pub tag: Unicase,
    pub url: UrlBuf,
}

impl<'de> Deserialize<'de> for Tag {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Tag, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(Tag {
            tag: Unicase(slug::slugify(&String::deserialize(deserializer)?)),
            url: UrlBuf::new(),
        })
    }
}

impl Tag {
    pub fn deserialize_seq<'de, D>(deserializer: D) -> std::result::Result<Vec<Tag>, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(Vec::deserialize(deserializer)?.into_iter().collect())
    }
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct Unicase(String);

impl Unicase {
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }

    pub fn equals(&self, other: &Unicase) -> bool {
        self.0 == other.0
    }
}

impl AsRef<Path> for Unicase {
    fn as_ref(&self) -> &Path {
        self.0.as_ref()
    }
}

impl Default for Unicase {
    fn default() -> Self {
        Self(String::default())
    }
}

impl<'de> Deserialize<'de> for Unicase {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Unicase, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(Unicase(String::deserialize(deserializer)?.to_lowercase()))
    }
}

impl From<Unicase> for String {
    fn from(unicase: Unicase) -> String {
        unicase.0
    }
}

impl From<&'_ Unicase> for String {
    fn from(unicase: &Unicase) -> String {
        unicase.0.clone()
    }
}

impl Display for Unicase {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

/// Summary represents a summarized post body.
#[derive(Clone)]
pub struct Summary {
    /// `url` is a reference to the full post.
    pub url: UrlBuf,

    /// `summary` is the body up to the `<!-- more -->` tag. If there is no such
    /// tag, then `summary` is the full post body. Whether or not the tag was
    /// found is stored as a boolean in [`summarized`].
    pub summary: String,

    /// `summarized` indicates whether or not a `<!-- more -->` tag was found.
    /// This is useful for displaying an optional "Read More" link.
    pub summarized: bool,
}

#[derive(Deserialize, Clone)]
pub struct RawPost(_Post<String, Unicase>);

pub type PostSummary = _Post<Summary, Tag>;

pub type Post = _Post<String, Tag>;

#[derive(Deserialize, Clone)]
pub struct _Post<B, T> {
    #[serde(default)]
    pub id: String,

    #[serde(rename = "Title")]
    pub title: String,

    #[serde(rename = "Date")]
    pub date: String,

    #[serde(default)]
    pub body: B,

    #[serde(default, rename = "Tags")]
    pub tags: Vec<T>,
}

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    FrontmatterMissingStartFence,
    FrontmatterMissingEndFence,
    DeserializeYaml(serde_yaml::Error),
    Io(std::io::Error),
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
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

impl Post {
    pub fn summarize(self, posts_base_url: &Url) -> PostSummary {
        PostSummary {
            title: self.title,
            date: self.date,
            body: match self.body.find("<!-- more -->") {
                Some(i) => Summary {
                    url: posts_base_url.join(&self.id),
                    summary: self.body[..i].to_owned(),
                    summarized: true,
                },
                None => Summary {
                    url: posts_base_url.join(&self.id),
                    summary: self.body,
                    summarized: false,
                },
            },
            id: self.id,
            tags: self.tags,
        }
    }
}

impl RawPost {
    pub fn finalize(self, tags_base_url: &Url) -> Post {
        Post {
            id: self.0.id,
            title: self.0.title,
            date: self.0.date,
            body: self.0.body,
            tags: self
                .0
                .tags
                .into_iter()
                .map(|t| Tag {
                    url: tags_base_url.join(format!("{}/index.html", &t)),
                    tag: t,
                })
                .collect(),
        }
    }

    pub fn from_str(id: &str, url: &Url, input: &str) -> Result<Self> {
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
        let mut post: RawPost = serde_yaml::from_str(&input[yaml_start..yaml_stop])?;
        post.0.id = id.to_owned();
        let mut options = Options::empty();
        options.insert(Options::ENABLE_FOOTNOTES);
        options.insert(Options::ENABLE_SMART_PUNCTUATION);
        options.insert(Options::ENABLE_STRIKETHROUGH);
        options.insert(Options::ENABLE_TABLES);
        options.insert(Options::ENABLE_TASKLISTS);
        let parser = Parser::new_ext(&input[body_start..], options);

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

        push_html(&mut post.0.body, fixed_subheading_sizes, url.as_str())?;
        Ok(post)
    }
}

fn parse_raw_posts<F>(
    directory: &Path,
    id_to_url: F,
) -> Result<impl Iterator<Item = Result<RawPost>>>
where
    F: FnOnce(&str) -> UrlBuf + Copy,
{
    use std::io::Read;
    const MARKDOWN_EXTENSION: &str = ".md";

    Ok(read_dir(directory)?.filter_map(move |result| match result {
        Err(e) => Some(Err(Error::Io(e))),
        Ok(entry) => {
            let os_file_name = entry.file_name();
            let file_name = os_file_name.to_string_lossy();
            if file_name.ends_with(MARKDOWN_EXTENSION) {
                let base_name = file_name.trim_end_matches(MARKDOWN_EXTENSION);
                let mut contents = String::new();
                match File::open(entry.path()) {
                    Err(e) => Some(Err(Error::Io(e))),
                    Ok(mut file) => match file.read_to_string(&mut contents) {
                        Err(e) => Some(Err(Error::Io(e))),
                        Ok(_) => Some(RawPost::from_str(
                            base_name,
                            &id_to_url(base_name),
                            &contents,
                        )),
                    },
                }
            } else {
                None
            }
        }
    }))
}

pub fn parse_posts<'a, F>(
    dir: &'a Path,
    id_to_url: F,
    tags_base_url: &'a Url,
) -> Result<impl Iterator<Item = Result<Post>> + 'a>
where
    F: FnOnce(&str) -> UrlBuf + Copy + 'a,
{
    Ok(
        parse_raw_posts(dir, id_to_url)?.map(move |result| match result {
            Err(e) => Err(e),
            Ok(p) => Ok(p.finalize(tags_base_url)),
        }),
    )
}

// Walks `dir` and returns a vector of posts ordered by date.
pub fn parse_posts_sorted<F>(dir: &Path, id_to_url: F, tags_base_url: &Url) -> Result<Vec<Post>>
where
    F: FnOnce(&str) -> UrlBuf + Copy,
{
    let mut posts = parse_posts(dir, id_to_url, tags_base_url)?.collect::<Result<Vec<Post>>>()?;
    posts.sort_by(|a, b| b.date.cmp(&a.date));
    Ok(posts)
}
