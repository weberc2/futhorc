use anyhow::{anyhow, Result};
use std::fs::File;
use std::path::Path;

pub fn open(path: &Path, kind: &str) -> Result<File> {
    match File::open(path) {
        Err(e) => Err(anyhow!("Opening {} file `{}`: {}", kind, path.display(), e)),
        Ok(file) => Ok(file),
    }
}
