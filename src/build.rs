use crate::config::Config;
use crate::page::*;
use crate::post::{self, *};
use crate::slice::*;
use crate::url::*;
use crate::write::{self, *};
use gtmpl::Template;
use std::collections::HashMap;
use std::fmt;
use std::path::{Path, PathBuf};

type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    Write(write::Error),
    Post(post::Error),
    Clean { path: PathBuf, err: std::io::Error },
    OpenTemplateFile { path: PathBuf, err: std::io::Error },
    ParseTemplate(String),
    Io(std::io::Error),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::Write(err) => err.fmt(f),
            Error::Post(err) => err.fmt(f),
            Error::Clean { path, err } => {
                write!(f, "Cleaning directory '{}': {}", path.display(), err)
            }
            Error::OpenTemplateFile { path, err } => {
                write!(f, "Opening template file '{}': {}", path.display(), err)
            }
            Error::ParseTemplate(err) => err.fmt(f),
            Error::Io(err) => err.fmt(f),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Write(err) => Some(err),
            Error::Post(err) => Some(err),
            Error::Clean { path: _, err } => Some(err),
            Error::OpenTemplateFile { path: _, err } => Some(err),
            Error::ParseTemplate(_) => None,
            Error::Io(err) => Some(err),
        }
    }
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Error {
        Error::Io(err)
    }
}

impl From<post::Error> for Error {
    fn from(err: post::Error) -> Error {
        Error::Post(err)
    }
}

impl From<write::Error> for Error {
    fn from(err: write::Error) -> Error {
        Error::Write(err)
    }
}

pub fn build_site(config: &Config) -> Result<()> {
    fn rmdir(dir: &Path) -> Result<()> {
        match std::fs::remove_dir_all(dir) {
            Ok(x) => Ok(x),
            Err(e) => match e.kind() {
                std::io::ErrorKind::NotFound => Ok(()),
                _ => Err(Error::Clean {
                    path: dir.to_owned(),
                    err: e,
                }),
            },
        }
    }
    // collect all posts
    let posts: Vec<Post<Tag>> = parse_posts(&config.posts_source_directory, move |id| {
        config.posts_url.join(format!("{}.html", id))
    })?
    .into_iter()
    .map(|p| p.convert_tags(&config.index_url))
    .collect();

    // Parse the template files.
    let index_template = parse_template(config.index_template.iter())?;
    let posts_template = parse_template(config.posts_template.iter())?;

    // Blow away the old output directories (if they exists) so we don't have any collisions
    rmdir(&config.index_output_directory)?;
    rmdir(&config.posts_output_directory)?;
    rmdir(&config.static_output_directory)?;

    // render index pages
    render_indices(
        &build_indices(&posts),
        &config.index_url,
        &config.index_output_directory,
        &index_template,
        &config.posts_url,
        &config.home_page,
        &config.static_url,
        config.index_page_size,
        config.threads,
    )?;

    // render post pages
    write_pages(
        post_pages(&posts, &config.posts_url).into_iter(),
        &config.posts_output_directory,
        &posts_template,
        &config.home_page,
        &config.static_url,
        config.threads,
    )?;

    // copy static directory
    copy_dir(
        &config.static_source_directory,
        &config.static_output_directory,
    )
}

fn copy_dir(src: &Path, dst: &Path) -> Result<()> {
    std::fs::create_dir(dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        if entry.file_type()?.is_dir() {
            copy_dir(src, &dst.join(entry.file_name()))?;
        } else {
            std::fs::copy(src.join(entry.file_name()), dst.join(entry.file_name()))?;
        }
    }

    Ok(())
}

fn render_indices(
    indices: &HashMap<String, Vec<&Post<Tag>>>,
    index_url: &Url,
    index_directory: &Path,
    index_template: &Template,
    posts_url: &Url,
    home_page: &Url,
    static_root: &Url,
    page_size: usize,
    threads: usize,
) -> Result<()> {
    for (tag, index) in indices {
        render_index(
            index,
            tag,
            posts_url,
            index_url,
            index_directory,
            index_template,
            home_page,
            static_root,
            page_size,
            threads,
        )?;
    }
    Ok(())
}

fn render_index(
    index: &[&Post<Tag>],
    tag: &str,
    posts_url: &Url,
    index_url: &Url,
    index_directory: &Path,
    index_template: &Template,
    home_page: &Url,
    static_root: &Url,
    page_size: usize,
    threads: usize,
) -> Result<()> {
    Ok(write_pages(
        index_pages(
            &index
                .into_iter()
                .map(|p| PostSummary::from((*p, posts_url)))
                .collect::<Vec<PostSummary>>(),
            page_size,
            index_url,
        ),
        &index_directory.join(&tag),
        index_template,
        home_page,
        static_root,
        threads,
    )?)
}

fn post_pages<'a>(posts: &'a [Post<Tag>], base_url: &Url) -> Vec<Page<&'a Post<Tag>>> {
    match posts.len() {
        0 => Vec::new(),
        1 => vec![Page {
            item: &posts[0],
            id: posts[0].id.clone(),
            prev: None,
            next: None,
        }],
        _ => std::iter::once(Page {
            item: &posts[0],
            id: posts[0].id.clone(),
            prev: None,
            next: Some(base_url.join(format!("{}.html", &posts[1].id))),
        })
        .chain(posts.array_windows().map(|[prev, post, next]| Page {
            item: post,
            id: post.id.clone(),
            prev: Some(base_url.join(format!("{}.html", &prev.id))),
            next: Some(base_url.join(format!("{}.html", &next.id))),
        }))
        .chain(std::iter::once(Page {
            item: &posts[posts.len() - 1],
            id: posts[posts.len() - 1].id.clone().into(),
            prev: Some(base_url.join(format!("{}.html", posts[posts.len() - 2].id))),
            next: None,
        }))
        .collect(),
    }
}

fn index_pages<'a>(
    posts: &'a [PostSummary],
    page_size: usize,
    base_url: &'a Url,
) -> impl Iterator<Item = Page<Slice<'a, PostSummary>>> {
    fn identify(page_number: usize) -> String {
        match page_number {
            0 => "index".to_string(),
            _ => page_number.to_string(),
        }
    }

    fn identify_url(base_url: &Url, page_number: usize) -> UrlBuf {
        base_url.join(format!("{}.html", identify(page_number)))
    }

    let total_pages = match posts.len() % page_size {
        0 => posts.len() / page_size,
        _ => posts.len() / page_size + 1,
    };
    posts
        .chunks(page_size)
        .enumerate()
        .map(move |(page_number, posts)| Page {
            item: Slice::new(posts),
            id: identify(page_number),
            prev: match page_number {
                0 => None,
                _ => Some(identify_url(base_url, page_number - 1)),
            },
            next: match page_number + 1 < total_pages {
                false => None,
                true => Some(identify_url(base_url, page_number + 1)),
            },
        })
}

fn build_indices<'a>(posts: &'a [Post<Tag>]) -> HashMap<String, Vec<&'a Post<Tag>>> {
    let mut m: HashMap<String, Vec<&'a Post<Tag>>> = HashMap::new();
    for post in posts {
        for tag in post.tags.iter() {
            match m.get_mut(&tag.tag) {
                None => {
                    m.insert(tag.tag.clone(), vec![post]);
                }
                Some(posts) => {
                    posts.push(post);
                }
            };
        }
    }
    // Include a "main index" consisting of all of the posts whose "tag" is the
    // empty string.
    m.insert(String::new(), posts.iter().collect());
    m
}

// Loads the template file contents, appends them to `base_template`, and
// parses the result into a template.
fn parse_template<'a, P: AsRef<Path>>(template_files: impl Iterator<Item = P>) -> Result<Template> {
    let mut contents = String::new();
    for template_file in template_files {
        use std::fs::File;
        let template_file = template_file.as_ref();
        File::open(template_file).map_err(|e| Error::OpenTemplateFile {
            path: template_file.to_owned(),
            err: e,
        })?;
        contents.push(' ');
    }
    let mut template = Template::default();
    template.parse(&contents).map_err(Error::ParseTemplate)?;
    Ok(template)
}
