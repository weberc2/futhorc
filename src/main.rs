#![feature(array_windows)]

use crate::build::*;
use crate::url::*;
use std::path::PathBuf;

mod build;
mod page;
mod post;
mod slice;
mod url;
mod value;
mod write;

fn main() {
    build_site(&Config {
        source_directory: PathBuf::from(match &*std::env::args().collect::<Vec<String>>() {
            [_, path, ..] => path.as_str(),
            _ => "./test-data/posts/",
        }),
        site_root: UrlBuf::from("file:///tmp/pages/0.html"),
        index_url: UrlBuf::from("file:///tmp/pages"),
        index_template: PathBuf::from("./index-template.html"),
        index_directory: PathBuf::from("/tmp/pages"),
        index_page_size: 10,
        posts_url: UrlBuf::from("file:///tmp/posts"),
        posts_template: PathBuf::from("./post-template.html"),
        posts_directory: PathBuf::from("/tmp/posts"),
        threads: None,
    })
    .unwrap();
}
