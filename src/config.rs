//! Contains the logic for collecting and consolidating the program's
//! configuration.

use crate::url::UrlBuf;
use serde::Deserialize;
use std::fmt;
use std::fs::File;
use std::path::{Path, PathBuf};

#[derive(Deserialize)]
struct PageSize(usize);
impl Default for PageSize {
    fn default() -> Self {
        PageSize(10)
    }
}

#[derive(Deserialize)]
struct Project {
    #[serde(default)]
    pub root_directory: PathBuf,
    pub site_root: UrlBuf,
    pub home_page: UrlBuf,

    #[serde(default)]
    pub index_page_size: PageSize,
}

#[derive(Deserialize)]
struct Theme {
    index_template: Vec<PathBuf>,
    posts_template: Vec<PathBuf>,
}

/// The complete configuration object, ready to be passed to
/// [`crate::build::build_site`].
#[derive(Debug)]
pub struct Config {
    /// The absolute path to the directory in which the post source files (`.md`)
    /// are located.
    pub posts_source_directory: PathBuf,

    /// The fully-qualified URL to the site's home page. This comes from the
    /// `futhorc.yaml` project file and is intended to be provided to the index
    /// and post templates, e.g., to create a site-header link.
    pub home_page: UrlBuf,

    /// The fully-qualified base URL for the index pages. The main index pages
    /// will live at `{index_url}/index.html`, `{index_url}/1.html`, etc. The tag
    /// index pages will live at `{index_url}/{tag_name}/index.html`,
    /// `{index_url}/{tag_name}/1.html`, etc.
    pub index_url: UrlBuf,

    /// The paths to index template files which will be concatenated and the result
    /// parsed into a [`gtmpl::Template`] object.
    pub index_template: Vec<PathBuf>,

    /// The absolute path to the output directory for index files. The main index
    /// page files will live at `{index_output_directory}/index.html`,
    /// `{index_output_directory}/1.html`, etc. The tag index page files will
    /// live at `{index_output_directory}/{tag_name}/index.html`,
    /// `{index_output_directory}/{tag_name}/1.html`, etc.
    pub index_output_directory: PathBuf,

    /// The number of posts per index page. Defaults to 10.
    pub index_page_size: usize,

    /// The fully-qualified base URL for post pages. E.g., for a post whose
    /// source file is located at `{posts_source_directory}/foo/bar.md`, the URL
    /// will be `{posts_url}/foo/bar.html`.
    pub posts_url: UrlBuf,

    /// The paths to post template files which will be concatenated and the result
    /// parsed into a [`gtmpl::Template`] object.
    pub posts_template: Vec<PathBuf>,

    /// The fully-qualified base URL for post pages. E.g., for a post whose
    /// source file is located at `{posts_source_directory}/foo/bar.md`, the URL
    /// will be `{posts_url}/foo/bar.html`.
    pub posts_output_directory: PathBuf,

    /// The fully-qualified base URL for static assets. E.g., a static asset
    /// whose source file is located at `{static_source_directory}/style.css`
    /// will have the URL, `{static_url}/style.css`.
    pub static_url: UrlBuf,

    /// The absolute path to the source directory for static assets.
    pub static_source_directory: PathBuf,

    /// The absolute path to the output directory for static assets.
    pub static_output_directory: PathBuf,
}

/// The result type for fallible configuration operations, namely parsing
/// configuration files.
type Result<T> = std::result::Result<T, Error>;

/// The error type for fallible configuration operations, namely parsing
/// configuration files.
#[derive(Debug)]
pub enum Error {
    /// Returned when the project file can't be found.
    MissingProjectFile(PathBuf),

    /// Returned when the project directory can't be determined.
    MissingProjectDirectory(PathBuf),

    /// Returned when the configuration files are malformed.
    DeserializeYaml(serde_yaml::Error),

    /// Returned when there is a problem opening a theme file.
    OpenThemeFile { path: PathBuf, err: std::io::Error },

    /// Returned when there is a problem opening the project file.
    OpenProjectFile { path: PathBuf, err: std::io::Error },

    /// Returned for other I/O errors.
    Io(std::io::Error),
}

impl fmt::Display for Error {
    /// Implements [`std::fmt::Display`] for [`Error`].
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::MissingProjectFile(dir) => write!(
                f,
                "Could not find `futhorc.yaml` in any parent directory of '{}'",
                dir.display()
            ),
            Error::MissingProjectDirectory(path) => write!(
                f,
                "Couldn't locate parent directory for provided file path '{}'",
                path.display()
            ),
            Error::DeserializeYaml(err) => err.fmt(f),
            Error::OpenThemeFile { path, err } => {
                write!(f, "Opening theme file '{}': {}", path.display(), err,)
            }
            Error::OpenProjectFile { path, err } => {
                write!(f, "Opening project file '{}': {}", path.display(), err)
            }
            Error::Io(err) => err.fmt(f),
        }
    }
}

impl std::error::Error for Error {
    /// Implements [`std::error::Error`] for [`Error`].
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::MissingProjectFile(_) => None,
            Error::MissingProjectDirectory(_) => None,
            Error::DeserializeYaml(err) => Some(err),
            Error::OpenThemeFile { path: _, err } => Some(err),
            Error::OpenProjectFile { path: _, err } => Some(err),
            Error::Io(err) => Some(err),
        }
    }
}

impl From<serde_yaml::Error> for Error {
    /// Converts [`serde_yaml::Error`] into [`Error`]. This allows us to use the
    /// `?` operator on fallible config parsing operations.
    fn from(err: serde_yaml::Error) -> Error {
        Error::DeserializeYaml(err)
    }
}

impl From<std::io::Error> for Error {
    /// Converts [`std::io::Error`] into [`Error`]. This allows us to use the
    /// `?` operator on fallible config parsing operations.
    fn from(err: std::io::Error) -> Error {
        Error::Io(err)
    }
}

impl Config {
    /// Loads a [`Config`] object from source and output directory parameters.
    pub fn from_directory(dir: &Path, output_directory: &Path) -> Result<Config> {
        let path = dir.join("futhorc.yaml");
        if path.exists() {
            Config::from_project_file(&path, output_directory)
        } else {
            match dir.parent() {
                Some(dir_) => Config::from_directory(dir_, output_directory),
                None => Err(Error::MissingProjectFile(dir.to_owned())),
            }
        }
    }

    /// Loads a [`Config`] object from project file path and output directory
    /// path parameters.
    pub fn from_project_file(path: &Path, output_directory: &Path) -> Result<Config> {
        let project: Project =
            serde_yaml::from_reader(File::open(path).map_err(|e| Error::OpenProjectFile {
                path: path.to_owned(),
                err: e,
            })?)?;
        match path.parent() {
            None => Err(Error::MissingProjectDirectory(path.to_owned())),
            Some(project_root) => {
                let theme_dir = project_root.join("theme");
                let theme_path = theme_dir.join("theme.yaml");
                let theme_file = File::open(&theme_path).map_err(|e| Error::OpenThemeFile {
                    path: theme_path,
                    err: e,
                })?;
                let theme: Theme = serde_yaml::from_reader(theme_file)?;
                Ok(Config {
                    home_page: project.site_root.join(project.home_page),
                    posts_source_directory: project_root.join("posts"),
                    index_url: (&project.site_root).join("pages"),
                    posts_url: (&project.site_root).join("posts"),
                    index_template: theme
                        .index_template
                        .iter()
                        .map(|relpath| theme_dir.join(relpath))
                        .collect(),
                    posts_template: theme
                        .posts_template
                        .iter()
                        .map(|relpath| theme_dir.join(relpath))
                        .collect(),
                    index_output_directory: output_directory.join("pages"),
                    posts_output_directory: output_directory.join("posts"),
                    static_url: (&project.site_root).join("static"),
                    static_source_directory: theme_dir.join("static"),
                    static_output_directory: output_directory.join("static"),
                    index_page_size: project.index_page_size.0,
                })
            }
        }
    }
}
