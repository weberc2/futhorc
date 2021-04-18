use crate::post::*;
use crate::url::{Url, UrlBuf};
use gtmpl::{Template, Value};
use std::fmt;
use std::io;
use std::path::{Path, PathBuf};

/// Responsible for indexing, templating, and writing HTML pages to disk from
/// [`Post`] sources.
pub struct Writer<'a> {
    /// The template for post pages.
    pub posts_template: &'a Template,

    /// The template for index pages.
    pub index_template: &'a Template,

    /// The base URL for index pages. The main index pages will be located at
    /// `{index_base_url}/index.html`, `{index_base_url/1.html}`, etc. The tag
    /// index pages will be located at `{index_base_url}/{tag_name}/index.html`,
    /// `{index_base_url}/{tag_name}/1.html`, etc.
    pub index_base_url: &'a Url,

    /// The directory in which the index HTML files will be written. The main
    /// index page files will be located at
    /// `{index_output_directory}/index.html`, `{index_output_directory}/1.html`,
    /// etc. The tag index page files will be located at
    /// `{index_output_directory}/{tag_name}/index.html`,
    /// `{index_output_directory}/{tag_name}/1.html`,
    pub index_output_directory: &'a Path,

    /// The number of posts per index page.
    pub index_page_size: usize,

    /// The URL for the site's home page. This is made available to both post and
    /// index templates, typically as the destination for the site-header link.
    pub home_page: &'a Url,

    /// The URL for the static assets. This is made available to both post and
    /// index templates, typically for the theme's stylesheet.
    pub static_url: &'a Url,
}

impl Writer<'_> {
    /// Takes a single [`Page`], templates it, and writes it to disk.
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

    /// Takes a slice of [`Post`], indexes it by tag, and writes post and index
    /// pages to disk.
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

/// An object representing an output HTML file. A [`Page`] can be converted to a
/// [`Value`] and thus rendered in a template via [`Page::to_value`].
struct Page<'a> {
    /// The main item for the page.
    item: Value,

    /// The target location on disk for the output file.
    file_path: PathBuf,

    /// The URL for the previous page, if any.
    prev: Option<UrlBuf>,

    /// The URL for the next page, if any.
    next: Option<UrlBuf>,

    /// The template with which the page will be rendered.
    template: &'a Template,
}

impl Page<'_> {
    /// Converts a [`Page`] into a [`Value`]. The result is a [`Value::Object`]
    /// with fields `item`, `prev`, and `next` (see [`Page`] for descriptions).
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

/// Creates all of the index and post [`Page`]s for a set of `[Post]`s. See
/// `[Writer]` for a description of arguments. Calls [`index_pages`] and
/// [`post_pages`] and returns the union of their results as a single stream of [`Page`]s.
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

/// Creates all of the post [`Page`]s for a set of [`Post`]s. Takes the posts and
/// the post template as arguments.
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

/// Creates all of the index [`Page`]s for a set of [`Post`]s. Takes the posts
/// and various `index_` parameters. See [`Writer`] for descriptions of the
/// `index_` parameters.
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

/// `Index` represents a collection of [`Post`]s associated with tag (including
/// the empty tag, which is the main index containing all posts).
struct Index<'a> {
    /// The base URL for all posts in the index.
    url: UrlBuf,

    /// The output directory for all posts in the index.
    output_directory: PathBuf,

    /// The posts associated with the index.
    posts: Vec<&'a Post>,
}

impl<'a, 't> Index<'a> {
    /// Converts the index to a list of index pages. `index_page_size` and
    /// `index_template` represent the number of posts per page and the template
    /// to apply to the pages respectively.
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

/// Indexes a list of [`Post`] objects.
///
/// Arguments:
///
/// * `base_url` is the base URL for index pages. See [`Writer::index_base_url`]
///   for more details.
/// * `base_directory` is the base directory for index pages. See
///   [`Writer::index_output_directory`] for more details.
/// * `posts` is the collection of [`Post`] objects to index.
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

/// The result of a fallible page-writing operation.
type Result<T> = std::result::Result<T, Error>;

/// Represents an error in a page-writing operation.
#[derive(Debug)]
pub enum Error {
    /// An error during templating.
    Template(String),

    /// An error writing the output files.
    Io(io::Error),
}

impl From<io::Error> for Error {
    /// Converts an [`io::Error`] into an [`Error`]. This allows us to use the
    /// `?` operator for fallible I/O operations.
    fn from(err: io::Error) -> Error {
        Error::Io(err)
    }
}

impl From<String> for Error {
    /// Converts a template error message ([`String`]) into an [`Error`]. This
    /// allows us to use the `?` operator for fallible template operations.
    fn from(err: String) -> Error {
        Error::Template(err)
    }
}

impl fmt::Display for Error {
    /// Displays an [`Error`] as presentable text.
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::Template(err) => err.fmt(f),
            Error::Io(err) => err.fmt(f),
        }
    }
}

impl std::error::Error for Error {
    /// Implements the [`std::error::Error`] trait for [`Error`].
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Template(_) => None,
            Error::Io(err) => Some(err),
        }
    }
}
