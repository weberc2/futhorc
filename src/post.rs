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
    pub tag: String,
    pub url: UrlBuf,
}

impl<'de> Deserialize<'de> for Tag {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Tag, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(Tag {
            tag: slug::slugify(&String::deserialize(deserializer)?),
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

pub struct Unicase(String);

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

#[derive(Deserialize)]
pub struct Post<T> {
    #[serde(default)]
    pub id: String,

    #[serde(rename = "Title")]
    pub title: String,

    #[serde(rename = "Date")]
    pub date: String,

    #[serde(default)]
    pub body: String,

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

impl Post<Unicase> {
    pub fn convert_tags(&self, tags_base_url: &Url) -> Post<Tag> {
        Post {
            id: self.id.clone(),
            title: self.title.clone(),
            date: self.date.clone(),
            body: self.body.clone(),
            tags: self
                .tags
                .iter()
                .map(|t| Tag {
                    tag: t.into(),
                    url: tags_base_url.join(format!("{}/index.html", t)),
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
        let mut post: Post<Unicase> = serde_yaml::from_str(&input[yaml_start..yaml_stop])?;
        post.id = id.to_owned();
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

        push_html(&mut post.body, fixed_subheading_sizes, url.as_str())?;
        Ok(post)
    }
}

impl<T> Post<T> {
    pub fn summary(&self) -> (&str, bool) {
        const FOLD_TAG: &str = "<!-- more -->";
        match self.body.find(FOLD_TAG) {
            Some(i) => (&self.body[..i], true),
            None => (&self.body, false),
        }
    }
}

const MARKDOWN_EXTENSION: &str = ".md";

fn process_entry<F>(file_name: &str, full_path: &Path, id_to_url: F) -> Result<Post<Unicase>>
where
    F: FnOnce(&str) -> UrlBuf,
{
    use std::io::Read;

    let base_name = file_name.trim_end_matches(MARKDOWN_EXTENSION);
    let mut contents = String::new();
    File::open(full_path)?.read_to_string(&mut contents)?;
    Post::from_str(base_name, &id_to_url(base_name), &contents)
}

// Walks `dir` and returns a vector of posts ordered by date.
pub fn parse_posts<F>(dir: &Path, id_to_url: F) -> Result<Vec<Post<Unicase>>>
where
    F: FnOnce(&str) -> UrlBuf + Copy,
{
    let mut posts: Vec<Post<Unicase>> = Vec::new();

    for result in read_dir(dir)? {
        let entry = result?;
        let os_file_name = entry.file_name();
        let file_name = os_file_name.to_string_lossy();
        if file_name.ends_with(MARKDOWN_EXTENSION) {
            posts.push(process_entry(&file_name, &entry.path(), id_to_url)?);
        }
    }

    posts.sort_by(|a, b| b.date.cmp(&a.date));

    Ok(posts)
}

#[derive(Clone)]
pub struct PostSummary {
    pub id: String,
    pub url: UrlBuf,
    pub title: String,
    pub date: String,
    pub summary: String,
    pub summarized: bool,
    pub tags: Vec<Tag>,
}

impl From<(&Post<Tag>, &Url)> for PostSummary {
    fn from(tuple: (&Post<Tag>, &Url)) -> PostSummary {
        let (p, base_url) = tuple;
        let (summary, summarized) = p.summary();
        PostSummary {
            id: p.id.clone(),
            url: base_url.join(&format!("{}.html", p.id)),
            title: p.title.clone(),
            date: p.date.clone(),
            summary: summary.to_owned(),
            summarized: summarized,
            tags: p.tags.clone(),
        }
    }
}
