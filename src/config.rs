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

pub struct Config {
    pub posts_source_directory: PathBuf,
    pub home_page: UrlBuf,
    pub index_url: UrlBuf,
    pub index_template: Vec<PathBuf>,
    pub index_output_directory: PathBuf,
    pub index_page_size: usize,
    pub posts_url: UrlBuf,
    pub posts_template: Vec<PathBuf>,
    pub posts_output_directory: PathBuf,
    pub static_url: UrlBuf,
    pub static_source_directory: PathBuf,
    pub static_output_directory: PathBuf,
    pub threads: usize,
}

type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    MissingProjectFile(PathBuf),
    MissingProjectDirectory(PathBuf),
    DeserializeYaml(serde_yaml::Error),
    OpenThemeFile { path: PathBuf, err: std::io::Error },
    OpenProjectFile { path: PathBuf, err: std::io::Error },
    Io(std::io::Error),
}

impl fmt::Display for Error {
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
    fn from(err: serde_yaml::Error) -> Error {
        Error::DeserializeYaml(err)
    }
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Error {
        Error::Io(err)
    }
}

impl Config {
    pub fn from_directory(
        dir: &Path,
        output_directory: &Path,
        threads: Option<usize>,
    ) -> Result<Config> {
        let path = dir.join("futhorc.yaml");
        if path.exists() {
            Config::from_project_file(&path, output_directory, threads)
        } else {
            match path.parent() {
                Some(dir) => Config::from_directory(dir, output_directory, threads),
                None => Err(Error::MissingProjectFile(dir.to_owned())),
            }
        }
    }

    pub fn from_project_file(
        path: &Path,
        output_directory: &Path,
        threads: Option<usize>,
    ) -> Result<Config> {
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
                    threads: match threads {
                        None => num_cpus::get(),
                        Some(threads) => threads,
                    },
                })
            }
        }
    }
}
