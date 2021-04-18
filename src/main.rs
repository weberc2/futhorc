use clap::{App, Arg, SubCommand};
use futhorc::build::{self, build_site};
use futhorc::config::{self, Config};
use std::fmt;
use std::path::{Path, PathBuf};

fn main() -> Result<(), Error> {
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
                )
                .arg(
                    Arg::with_name("PROFILE")
                        .long("profile")
                        .takes_value(true)
                        .required(false)
                        .value_name("PROFILE")
                        .help("The project profile to use for the build"),
                ),
        )
        .get_matches();

    if let Some(matches) = matches.subcommand_matches("build") {
        let cwd = std::env::current_dir().map_err(Error::Env)?;
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

        let config = Config::from_directory(project, &output, matches.value_of("PROFILE"))
            .map_err(Error::Config);
        return build_site(&config?).map_err(Error::Build);
    }
    Err(Error::MissingSubcommand)
}

/// Error is the toplevel error type, with variants for issues loading the
/// config, building the site, and general argument parsing.
enum Error {
    /// `MissingSubcommand` is represents a missing or incorrect subcommand.
    MissingSubcommand,

    /// `Config` represents errors loading the configuration.
    Config(config::Error),

    /// `Build` represents errors building the static site.
    Build(build::Error),

    /// `Env` represents errors parsing arguments from the process's environment.
    Env(std::io::Error),
}

impl fmt::Display for Error {
    /// `fmt` renders the error in text form.
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::MissingSubcommand => write!(f, "Missing subcommand. Try rerunning with --help"),
            Error::Config(err) => err.fmt(f),
            Error::Build(err) => err.fmt(f),
            Error::Env(err) => err.fmt(f),
        }
    }
}

// Implement Debug by invoking Display::fmt for the error so the toplevel error
// messages are prettier.
impl fmt::Debug for Error {
    /// `fmt` renders the error in text form. This just calls [`fmt::Display::fmt`] so
    /// the toplevel errors are prettier.
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}
