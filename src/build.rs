use crate::config::Config;
use crate::post::{Error as ParseError, Parser as PostParser};
use crate::write::{Error as WriteError, *};
use gtmpl::Template;
use std::fmt;
use std::path::{Path, PathBuf};

type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    Parse(ParseError),
    Write(WriteError),
    Clean { path: PathBuf, err: std::io::Error },
    OpenTemplateFile { path: PathBuf, err: std::io::Error },
    ParseTemplate(String),
    Io(std::io::Error),
}

impl fmt::Display for Error {
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
            Error::Io(err) => err.fmt(f),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Parse(err) => Some(err),
            Error::Write(err) => Some(err),
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

impl From<ParseError> for Error {
    fn from(err: ParseError) -> Error {
        Error::Parse(err)
    }
}

impl From<WriteError> for Error {
    fn from(err: WriteError) -> Error {
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

    // Blow away the old output directories (if they exists) so we don't have any collisions
    rmdir(&config.index_output_directory)?;
    rmdir(&config.posts_output_directory)?;
    rmdir(&config.static_output_directory)?;

    let writer = Writer {
        posts_template: &posts_template,
        index_template: &index_template,
        index_page_size: config.index_page_size,
        index_base_url: &config.index_url,
        index_output_directory: &config.index_output_directory,
        home_page: &config.home_page,
        static_url: &config.static_url,
    };
    writer.write_posts(&posts)?;

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

// Loads the template file contents, appends them to `base_template`, and
// parses the result into a template.
fn parse_template<'a, P: AsRef<Path>>(template_files: impl Iterator<Item = P>) -> Result<Template> {
    let mut contents = String::new();
    for template_file in template_files {
        use std::fs::File;
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
