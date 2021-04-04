#![feature(array_windows)]

use crate::build::*;
use crate::config::Config;
use anyhow::Result;
use clap::{App, Arg, SubCommand};
use std::path::{Path, PathBuf};

mod build;
mod config;
mod page;
mod post;
mod slice;
mod url;
mod util;
mod value;
mod write;

fn main() -> Result<()> {
    const DEFAULT_PROJECT_DIRECTORY: &str = "$PWD";
    const DEFAULT_OUTPUT_DIRECTORY: &str = "$PWD/_output";

    let matches = App::new("futhorc")
        .version("0.1")
        .author("Craig Weber <weberc2@gmail.com>")
        .about("A rustic static site generator")
        .subcommand(
            SubCommand::with_name("build")
                .about("Builds the static site")
                .arg(
                    Arg::with_name("PROJECT_DIRECTORY")
                        .short("p")
                        .long("project")
                        .required(true)
                        .takes_value(true)
                        .value_name("PROJECT_DIRECTORY")
                        .help("Any directory inside of the project to build")
                        .default_value(DEFAULT_PROJECT_DIRECTORY),
                )
                .arg(
                    Arg::with_name("OUTPUT_DIRECTORY")
                        .short("o")
                        .long("output")
                        .takes_value(true)
                        .required(true)
                        .value_name("OUTPUT_DIRECTORY")
                        .help("The target directory for the output HTML files")
                        .default_value(DEFAULT_OUTPUT_DIRECTORY),
                ),
        )
        .get_matches();

    if let Some(matches) = matches.subcommand_matches("build") {
        let cwd = std::env::current_dir()?;
        let project = matches
            .value_of("PROJECT_DIRECTORY")
            .expect("Argument PROJECT_DIRECTORY is required.");
        let project: &Path = match project {
            DEFAULT_PROJECT_DIRECTORY => &cwd,
            _ => Path::new(project),
        };

        let output = matches
            .value_of("OUTPUT_DIRECTORY")
            .expect("Argument OUTPUT_DIRECTORY is required");
        let output: PathBuf = match output {
            DEFAULT_OUTPUT_DIRECTORY => cwd.join("_output"),
            _ => PathBuf::from(output),
        };

        return build_site(&Config::from_directory(project, &output, None)?);
    }
    Err(anyhow::anyhow!(
        "Missing subcommand. Try rerunning with --help"
    ))
}
