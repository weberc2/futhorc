//! Exports the [`build_site`] function which stitches together the high-level
//! steps of building the output static site: parsing the posts
//! ([`crate::post`]), rendering index and post pages ([`crate::write`]), copying
//! the static source directory into the static output directory, and generating
//! the Atom feed.

use crate::config::Config;
use crate::feed::{Error as FeedError, *};
use crate::post::{Error as ParseError, Parser as PostParser};
use crate::write::{Error as WriteError, *};
use gtmpl::Template;
use std::fmt;
use std::fs::File;
use std::path::{Path, PathBuf};

/// Builds the site from a [`Config`] object. This calls into
/// [`PostParser::parse_posts`], [`Writer::write_posts`], and
/// [`feed::write_feed`] which do the heavy-lifting. This function also copies
/// the static assets from source directory to the output directory.
pub fn build_site(config: Config) -> Result<()> {
    let post_parser = PostParser::new(
        &config.index_url,
        &config.posts_url,
        &config.posts_output_directory,
    );

    // collect all posts
    let posts = post_parser.parse_posts(&config.posts_source_directory)?;

    // Parse the template files.
    let index_template = parse_template(config.index_template.iter())?;
    let posts_template = parse_template(config.posts_template.iter())?;

    // Blow away the old output directories so we don't have any collisions. We
    // probably don't want to naively delete the whole root output directory in
    // case the user accidentally passes the wrong directory. In the future, we
    // could refuse to build in a directory that already exists unless it was
    // created by `futhorc`, in which case we would then delete and rebuild that
    // directory. In order to tell that the output directory was created by
    // futhorc, we could leave a `.futhorc` watermark file, possibly with the
    // identifier of the specific futhorc project.
    rmdir(&config.posts_output_directory)?;
    rmdir(&config.index_output_directory)?;
    rmdir(&config.static_output_directory)?;

    // write the post and index pages
    let writer = Writer {
        posts_template: &posts_template,
        index_template: &index_template,
        index_page_size: config.index_page_size,
        index_base_url: &config.index_url,
        index_output_directory: &config.index_output_directory,
        home_page: &config.home_page,
        static_url: &config.static_url,
        atom_url: &config.atom_url,
    };
    writer.write_posts(&posts)?;

    // copy static directory
    copy_dir(
        &config.static_source_directory,
        &config.static_output_directory,
    )?;

    // copy /pages/index.html to /index.html
    let _ = std::fs::copy(
        &config.index_output_directory.join("index.html"),
        &config.root_output_directory.join("index.html"),
    )?;

    // create the atom feed
    write_feed(
        FeedConfig {
            title: config.title,
            id: config.home_page.to_string(),
            author: config.author,
            home_page: config.home_page,
        },
        &posts,
        File::create(config.root_output_directory.join("feed.atom"))?,
    )?;

    Ok(())
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

// Loads the template file contents, appends them to `base_template`, and
// parses the result into a template.
fn parse_template<P: AsRef<Path>>(template_files: impl Iterator<Item = P>) -> Result<Template> {
    let mut contents = String::new();
    for template_file in template_files {
        use std::io::Read;
        let template_file = template_file.as_ref();
        File::open(&template_file)
            .map_err(|e| Error::OpenTemplateFile {
                path: template_file.to_owned(),
                err: e,
            })?
            .read_to_string(&mut contents)?;
        contents.push(' ');
    }

    let mut template = Template::default();
    template.parse(&contents).map_err(Error::ParseTemplate)?;
    Ok(template)
}

type Result<T> = std::result::Result<T, Error>;

/// The error type for building a site. Errors can be during parsing, writing,
/// cleaning output directories, parsing template files, and other I/O.
#[derive(Debug)]
pub enum Error {
    /// Returned for errors during parsing.
    Parse(ParseError),

    /// Returned for errors writing [`crate::post::Post`]s to disk as HTML files.
    Write(WriteError),

    /// Returned for I/O problems while cleaning output directories.
    Clean { path: PathBuf, err: std::io::Error },

    /// Returned for I/O problems while opening template files.
    OpenTemplateFile { path: PathBuf, err: std::io::Error },

    /// Returned for errors parsing template files.
    ParseTemplate(String),

    /// Returned for errors writing the feed.
    Feed(FeedError),

    /// Returned for other I/O errors.
    Io(std::io::Error),
}

impl fmt::Display for Error {
    /// Implements [`fmt::Display`] for [`Error`].
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::Parse(err) => err.fmt(f),
            Error::Write(err) => err.fmt(f),
            Error::Clean { path, err } => {
                write!(f, "Cleaning directory '{}': {}", path.display(), err)
            }
            Error::OpenTemplateFile { path, err } => {
                write!(f, "Opening template file '{}': {}", path.display(), err)
            }
            Error::ParseTemplate(err) => err.fmt(f),
            Error::Feed(err) => err.fmt(f),
            Error::Io(err) => err.fmt(f),
        }
    }
}

impl std::error::Error for Error {
    /// Implements [`std::error::Error`] for [`Error`].
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Parse(err) => Some(err),
            Error::Write(err) => Some(err),
            Error::Clean { path: _, err } => Some(err),
            Error::OpenTemplateFile { path: _, err } => Some(err),
            Error::ParseTemplate(_) => None,
            Error::Feed(err) => Some(err),
            Error::Io(err) => Some(err),
        }
    }
}

impl From<std::io::Error> for Error {
    /// Converts [`std::io::Error`]s into [`Error`]. This allows us to use the
    /// `?` operator.
    fn from(err: std::io::Error) -> Error {
        Error::Io(err)
    }
}

impl From<ParseError> for Error {
    /// Converts [`ParseError`]s into [`Error`]. This allows us to use the `?`
    /// operator.
    fn from(err: ParseError) -> Error {
        Error::Parse(err)
    }
}

impl From<WriteError> for Error {
    /// Converts [`WriteError`]s into [`Error`]. This allows us to use the `?`
    /// operator.
    fn from(err: WriteError) -> Error {
        Error::Write(err)
    }
}

impl From<FeedError> for Error {
    /// Converts [`FeedError`]s into [`Error`]. This allows us to use the `?`
    /// operator.
    fn from(err: FeedError) -> Error {
        Error::Feed(err)
    }
}

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
