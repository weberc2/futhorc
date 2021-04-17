use crate::post::*;
use crate::url::{Url, UrlBuf};
use gtmpl::{Template, Value};
use std::fmt;
use std::io;
use std::path::{Path, PathBuf};

pub struct Writer<'a> {
    pub posts_template: &'a Template,
    pub index_template: &'a Template,
    pub index_base_url: &'a Url,
    pub index_output_directory: &'a Path,
    pub index_page_size: usize,
    pub home_page: &'a Url,
    pub static_url: &'a Url,
}

impl Writer<'_> {
    fn write_page(&self, page: &Page) -> Result<()> {
        let mut value = page.to_value();
        if let Value::Object(obj) = &mut value {
            obj.insert(
                "home_page".to_owned(),
                Value::String(self.home_page.to_string()),
            );
            obj.insert(
                "static_url".to_owned(),
                Value::String(self.static_url.to_string()),
            );
        }
        page.template.execute(
            &mut std::fs::File::create(&page.file_path)?,
            &gtmpl::Context::from(value).unwrap(),
        )?;
        Ok(())
    }

    pub fn write_posts(&self, posts: &[Post]) -> Result<()> {
        use std::collections::HashSet;
        let mut seen_dirs: HashSet<PathBuf> = HashSet::new();
        pages(
            posts,
            self.index_base_url,
            self.index_output_directory,
            self.index_page_size,
            self.posts_template,
            self.index_template,
        )
        .map(|page| {
            let dir = page.file_path.parent().unwrap(); // there should always be a dir
            if seen_dirs.insert(dir.to_owned()) {
                std::fs::create_dir_all(dir)?;
            }
            self.write_page(&page)
        })
        .collect()
    }
}

struct Page<'a> {
    item: Value,
    file_path: PathBuf,
    prev: Option<UrlBuf>,
    next: Option<UrlBuf>,
    template: &'a Template,
}

impl Page<'_> {
    fn to_value(&self) -> Value {
        use std::collections::HashMap;

        let option_to_value = |opt: &Option<UrlBuf>| match opt {
            Some(url) => url.into(),
            None => Value::Nil,
        };

        let mut m: HashMap<String, Value> = HashMap::new();
        m.insert("item".to_owned(), self.item.clone());
        m.insert("prev".to_owned(), option_to_value(&self.prev));
        m.insert("next".to_owned(), option_to_value(&self.next));
        Value::Object(m)
    }
}

fn pages<'a>(
    posts: &'a [Post],
    index_base_url: &Url,
    index_output_directory: &Path,
    index_page_size: usize,
    posts_template: &'a Template,
    index_template: &'a Template,
) -> impl Iterator<Item = Page<'a>> {
    index_pages(
        posts,
        index_base_url,
        index_output_directory,
        index_page_size,
        index_template,
    )
    .chain(post_pages(posts, posts_template))
}

fn post_pages<'a>(posts: &'a [Post], template: &'a Template) -> impl Iterator<Item = Page<'a>> {
    posts.iter().enumerate().map(move |(i, post)| Page {
        item: post.to_value(),
        file_path: post.file_path.clone(),
        prev: match i < 1 {
            true => None,
            false => Some(posts[i - 1].url.clone()),
        },
        next: match i >= posts.len() - 1 {
            true => None,
            false => Some(posts[i + 1].url.clone()),
        },
        template: template,
    })
}

fn index_pages<'a>(
    posts: &'a [Post],
    index_base_url: &Url,
    index_output_directory: &Path,
    index_page_size: usize,
    index_template: &'a Template,
) -> impl Iterator<Item = Page<'a>> {
    let indices = index_posts(index_base_url, index_output_directory, posts);
    indices
        .into_iter()
        .flat_map(move |i| i.to_pages(index_page_size, index_template))
}

struct Index<'a> {
    url: UrlBuf,
    output_directory: PathBuf,
    posts: Vec<&'a Post>,
}

impl<'a, 't> Index<'a> {
    fn to_pages(&self, index_page_size: usize, index_template: &'t Template) -> Vec<Page<'t>> {
        let total_pages = match self.posts.len() % index_page_size {
            0 => self.posts.len() / index_page_size,
            _ => self.posts.len() / index_page_size + 1,
        };

        self.posts
            .chunks(index_page_size)
            .enumerate()
            .map(|(i, chunk)| {
                let file_name = match i > 0 {
                    false => String::from("index.html"),
                    true => format!("{}.html", i),
                };

                Page {
                    item: Value::Array(chunk.iter().map(|p| p.summarize()).collect()),
                    file_path: self.output_directory.join(&file_name),
                    prev: match i {
                        0 => None,
                        1 => Some(self.url.join("index.html")),
                        _ => Some(self.url.join(format!("{}.html", i - 1))),
                    },
                    next: match i < total_pages - 1 {
                        false => None,
                        true => Some(self.url.join(format!("{}.html", i + 1))),
                    },
                    template: index_template,
                }
            })
            .collect()
    }
}

fn index_posts<'a>(base_url: &Url, base_directory: &Path, posts: &'a [Post]) -> Vec<Index<'a>> {
    use std::collections::HashMap;

    let mut indices: HashMap<String, Index> = HashMap::new();
    indices.insert(
        String::default(),
        Index {
            url: base_url.to_owned(),
            output_directory: base_directory.to_owned(),
            posts: posts.iter().collect(),
        },
    );

    for post in posts {
        for tag in post.tags.iter() {
            match indices.get_mut(&tag.tag) {
                None => {
                    indices.insert(
                        tag.tag.to_owned(),
                        Index {
                            url: base_url.join(&tag.tag).join("index.html"),
                            output_directory: base_directory.join(&tag.tag),
                            posts: vec![post],
                        },
                    );
                }
                Some(index) => {
                    index.posts.push(post);
                }
            }
        }
    }

    indices.into_values().collect()
}

type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    Gtmpl(String),
    Io(io::Error),
}

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Error {
        Error::Io(err)
    }
}

impl From<String> for Error {
    fn from(err: String) -> Error {
        Error::Gtmpl(err)
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::Gtmpl(err) => err.fmt(f),
            Error::Io(err) => err.fmt(f),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Gtmpl(_) => None,
            Error::Io(err) => Some(err),
        }
    }
}
