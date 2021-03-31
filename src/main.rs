#![feature(array_windows)]

use crate::build::*;
use anyhow::Result;
use serde_yaml;

mod build;
mod page;
mod post;
mod slice;
mod url;
mod value;
mod write;

fn main() -> Result<()> {
    build_site(&serde_yaml::from_str(
        r#"
            source_directory: /Users/weberc2/projects/blog/posts
            site_root:        file:///tmp/pages/0.html
            index_url:        file:///tmp/pages
            index_template:   [./base-template.html, ./index-template.html]
            index_directory:  /tmp/pages
            index_page_size:  10
            posts_url:        file:///tmp/posts
            posts_template:   [./base-template.html, ./posts-template.html]
            posts_directory:  /tmp/posts
        "#,
    )?)
}
