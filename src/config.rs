//! Contains the logic for collecting and consolidating the program's
//! configuration.

use serde::Deserialize;
use std::fmt;
use std::fs::File;
use std::path::{Path, PathBuf};
use url::Url;

#[derive(Deserialize)]
struct PageSize(usize);
impl Default for PageSize {
    fn default() -> Self {
        PageSize(10)
    }
}

/// Represents an author.
#[derive(Debug, Clone, Deserialize)]
pub struct Author {
    /// The author's name.
    pub name: String,

    /// The author's email.
    pub email: Option<String>,
}

#[derive(Deserialize)]
struct Profile {
    pub name: String,
    pub site_root: Url,
    pub home_page: String,
    pub author: Option<Author>,
    pub title: String,

    #[serde(default)]
    pub index_page_size: PageSize,
}

#[derive(Deserialize)]
struct Project {
    profiles: Vec<Profile>,
    default: String,
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
    /// The title of the site.
    pub title: String,

    /// The author of the site.
    pub author: Option<Author>,

    /// The absolute path to the root output directory.
    pub root_output_directory: PathBuf,

    /// The absolute path to the directory in which the post source files
    /// (`.md`) are located.
    pub posts_source_directory: PathBuf,

    /// The fully-qualified URL to the site's home page. This comes from the
    /// `futhorc.yaml` project file and is intended to be provided to the
    /// index and post templates, e.g., to create a site-header link.
    pub home_page: Url,

    /// The fully-qualified base URL for the index pages. The main index pages
    /// will live at `{index_url}/index.html`, `{index_url}/1.html`, etc. The
    /// tag index pages will live at `{index_url}/{tag_name}/index.html`,
    /// `{index_url}/{tag_name}/1.html`, etc.
    pub index_url: Url,

    /// The paths to index template files which will be concatenated and the
    /// result parsed into a [`gtmpl::Template`] object.
    pub index_template: Vec<PathBuf>,

    /// The absolute path to the output directory for index files. The main
    /// index page files will live at
    /// `{index_output_directory}/index.html`, `{index_output_directory}/
    /// 1.html`, etc. The tag index page files will
    /// live at `{index_output_directory}/{tag_name}/index.html`,
    /// `{index_output_directory}/{tag_name}/1.html`, etc.
    pub index_output_directory: PathBuf,

    /// The number of posts per index page. Defaults to 10.
    pub index_page_size: usize,

    /// The fully-qualified base URL for post pages. E.g., for a post whose
    /// source file is located at `{posts_source_directory}/foo/bar.md`, the
    /// URL will be `{posts_url}/foo/bar.html`.
    pub posts_url: Url,

    /// The paths to post template files which will be concatenated and the
    /// result parsed into a [`gtmpl::Template`] object.
    pub posts_template: Vec<PathBuf>,

    /// The fully-qualified base URL for post pages. E.g., for a post whose
    /// source file is located at `{posts_source_directory}/foo/bar.md`, the
    /// URL will be `{posts_url}/foo/bar.html`.
    pub posts_output_directory: PathBuf,

    /// The fully-qualified base URL for static assets. E.g., a static asset
    /// whose source file is located at `{static_source_directory}/style.css`
    /// will have the URL, `{static_url}/style.css`.
    pub static_url: Url,

    /// The absolute path to the source directory for static assets.
    pub static_source_directory: PathBuf,

    /// The absolute path to the output directory for static assets.
    pub static_output_directory: PathBuf,

    /// The fully-qualified URL for the atom feed.
    pub atom_url: Url,

    /// The absolute path to the atom output file.
    pub atom_output_path: PathBuf,
}

impl Config {
    /// Loads a [`Config`] object from source and output directory parameters.
    pub fn from_directory(
        dir: &Path,
        output_directory: &Path,
        profile: Option<&str>,
    ) -> Result<Config> {
        let path = dir.join("futhorc.yaml");
        if path.exists() {
            Config::from_project_file(&path, output_directory, profile)
        } else {
            match dir.parent() {
                Some(dir_) => {
                    Config::from_directory(dir_, output_directory, profile)
                }
                None => Err(Error::MissingProjectFile(dir.to_owned())),
            }
        }
    }

    /// Loads a [`Config`] object from project file path and output directory
    /// path parameters.
    pub fn from_project_file(
        path: &Path,
        output_directory: &Path,
        profile: Option<&str>,
    ) -> Result<Config> {
        let project: Project =
            serde_yaml::from_reader(File::open(path).map_err(|e| {
                Error::OpenProjectFile {
                    path: path.to_owned(),
                    err: e,
                }
            })?)?;
        let requested_profile = match profile {
            Some(profile) => profile,
            None => &project.default,
        };

        let profile = match project
            .profiles
            .into_iter()
            .find(|p| p.name == requested_profile)
        {
            None => Err(Error::UnknownProfile(requested_profile.to_owned())),
            Some(p) => Ok(p),
        }?;
        match path.parent() {
            None => Err(Error::MissingProjectDirectory(path.to_owned())),
            Some(project_root) => {
                let theme_dir = project_root.join("theme");
                let theme_path = theme_dir.join("theme.yaml");
                let theme_file = File::open(&theme_path).map_err(|e| {
                    Error::OpenThemeFile {
                        path: theme_path,
                        err: e,
                    }
                })?;
                let theme: Theme = serde_yaml::from_reader(theme_file)?;
                Ok(Config {
                    title: profile.title,
                    author: profile.author,
                    root_output_directory: output_directory.to_owned(),
                    home_page: profile.site_root.join(&profile.home_page)?,
                    posts_source_directory: project_root.join("posts"),
                    index_url: (&profile.site_root).join("pages/")?,
                    posts_url: (&profile.site_root).join("posts/")?,
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
                    static_url: (&profile.site_root).join("static/")?,
                    static_source_directory: theme_dir.join("static"),
                    static_output_directory: output_directory.join("static"),
                    index_page_size: profile.index_page_size.0,
                    atom_url: profile.site_root.join("feed.atom")?,
                    atom_output_path: output_directory.join("feed.atom"),
                })
            }
        }
    }
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

    /// Returned when the requested profile doesn't exist in the
    /// `futhorc.yaml` project file.
    UnknownProfile(String),

    /// Returned when there is a problem opening a theme file.
    OpenThemeFile { path: PathBuf, err: std::io::Error },

    /// Returned when there is a problem opening the project file.
    OpenProjectFile { path: PathBuf, err: std::io::Error },

    /// Returned when there is a problem parsing URLs.
    UrlParse(url::ParseError),

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
            Error::UnknownProfile(requested_profile) => {
                write!(
                    f,
                    "Could not find profile '{}' in `futhorc.yaml`",
                    requested_profile
                )
            }
            Error::OpenThemeFile { path, err } => {
                write!(f, "Opening theme file '{}': {}", path.display(), err,)
            }
            Error::OpenProjectFile { path, err } => {
                write!(f, "Opening project file '{}': {}", path.display(), err)
            }
            Error::UrlParse(err) => err.fmt(f),
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
            Error::UnknownProfile(_) => None,
            Error::OpenThemeFile { path: _, err } => Some(err),
            Error::OpenProjectFile { path: _, err } => Some(err),
            Error::UrlParse(err) => Some(err),
            Error::Io(err) => Some(err),
        }
    }
}

impl From<url::ParseError> for Error {
    /// Converts [`url::ParseError`] into [`Error`]. This allows us to use
    /// the `?` operator on fallible config parsing operations.
    fn from(err: url::ParseError) -> Error {
        Error::UrlParse(err)
    }
}

impl From<serde_yaml::Error> for Error {
    /// Converts [`serde_yaml::Error`] into [`Error`]. This allows us to use
    /// the `?` operator on fallible config parsing operations.
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
