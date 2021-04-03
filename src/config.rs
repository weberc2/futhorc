use crate::url::UrlBuf;
use anyhow::{anyhow, Result};
use serde::Deserialize;
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
    pub index_directory: PathBuf,
    pub index_page_size: usize,
    pub posts_url: UrlBuf,
    pub posts_template: Vec<PathBuf>,
    pub posts_directory: PathBuf,
    pub threads: usize,
}

impl Config {
    pub fn from_directory(
        dir: &Path,
        output_directory: &Path,
        threads: Option<usize>,
    ) -> Result<Config> {
        let path = dir.join("futhorc.yaml");
        if path.exists() {
            match Config::from_project_file(&path, output_directory, threads) {
                Ok(config) => Ok(config),
                Err(e) => Err(anyhow!("Loading configuration: {:?}", e)),
            }
        } else {
            match path.parent() {
                Some(dir) => Config::from_directory(dir, output_directory, threads),
                None => Err(anyhow!(
                    "Could not find `futhorc.yaml` in any parent directory"
                )),
            }
        }
    }

    pub fn from_project_file(
        path: &Path,
        output_directory: &Path,
        threads: Option<usize>,
    ) -> Result<Config> {
        use crate::util::open;
        let project: Project = serde_yaml::from_reader(open(path, "project")?)?;
        match path.parent() {
            None => Err(anyhow!(
                "Can't get parent directory for provided project file path '{:?}'",
                path
            )),
            Some(project_root) => {
                let theme_dir = project_root.join("theme");
                let theme_file = open(&theme_dir.join("theme.yaml"), "theme")?;
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
                    index_directory: output_directory.join("pages"),
                    posts_directory: output_directory.join("posts"),
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
