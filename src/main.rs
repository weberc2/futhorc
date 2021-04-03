#![feature(array_windows)]

use crate::build::*;
use crate::config::Config;
use anyhow::Result;
use std::path::Path;

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
    let args = std::env::args().collect::<Vec<String>>();
    let cwd = std::env::current_dir()?;
    let dir = if args.len() > 1 {
        Path::new(&args[1])
    } else {
        &cwd
    };
    build_site(&Config::from_directory(dir, &cwd.join("_output"), None)?)
}
