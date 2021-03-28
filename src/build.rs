use crate::page::*;
use crate::post::*;
use crate::slice::*;
use crate::url::*;
use crate::write::*;
use anyhow::Result;
use serde::Deserialize;
use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};

#[derive(Deserialize)]
pub struct Config {
    pub source_directory: PathBuf,
    pub site_root: UrlBuf,
    pub index_url: UrlBuf,
    pub index_template: PathBuf,
    pub index_directory: PathBuf,
    pub index_page_size: usize,
    pub posts_url: UrlBuf,
    pub posts_template: PathBuf,
    pub posts_directory: PathBuf,
    pub threads: Option<usize>,
}

fn read_to_string(path: &Path) -> Result<String> {
    let mut contents = String::new();
    File::open(path)?.read_to_string(&mut contents)?;
    Ok(contents)
}

pub fn build_site(config: &Config) -> Result<()> {
    // collect all posts
    let posts: Vec<Post<Tag>> = parse_posts(
        &config.source_directory,
        match config.threads {
            Some(threads) => threads,
            None => 1,
        },
    )?
    .into_iter()
    .map(|p| p.convert_tags(&config.index_url))
    .collect();

    let threads = match config.threads {
        Some(threads) => threads,
        None => 1,
    };

    // render index pages
    render_indices(
        &build_indices(&posts),
        &config.index_url,
        &config.index_directory,
        &read_to_string(&config.index_template)?,
        &config.site_root,
        config.index_page_size,
        threads,
    )?;

    // render post pages
    write_pages(
        post_pages(&posts, &config.posts_url).into_iter(),
        &config.posts_directory,
        &read_to_string(&config.posts_template)?,
        &config.site_root,
        threads,
    )
}

fn render_indices(
    indices: &HashMap<String, Vec<&Post<Tag>>>,
    index_url: &Url,
    index_directory: &Path,
    index_template: &str,
    site_root: &Url,
    page_size: usize,
    threads: usize,
) -> anyhow::Result<()> {
    for (tag, index) in indices {
        render_index(
            index,
            tag,
            index_url,
            index_directory,
            index_template,
            site_root,
            page_size,
            threads,
        )?;
    }
    Ok(())
}

fn render_index(
    index: &[&Post<Tag>],
    tag: &str,
    index_url: &Url,
    index_directory: &Path,
    index_template: &str,
    site_root: &Url,
    page_size: usize,
    threads: usize,
) -> anyhow::Result<()> {
    write_pages(
        index_pages(
            &index
                .into_iter()
                .map(|p| PostSummary::from((*p, index_url.join(tag).as_str())))
                .collect::<Vec<PostSummary>>(),
            page_size,
            index_url,
        ),
        &index_directory.join(&tag),
        index_template,
        site_root,
        threads,
    )
}

fn post_pages<'a>(posts: &'a [Post<Tag>], base_url: &Url) -> Vec<Page<&'a Post<Tag>>> {
    match posts.len() {
        0 => Vec::new(),
        1 => vec![Page {
            item: &posts[0],
            id: posts[0].id.clone(),
            prev: None,
            next: None,
        }],
        _ => std::iter::once(Page {
            item: &posts[0],
            id: posts[0].id.clone(),
            prev: None,
            next: Some(base_url.join(&posts[1].id)),
        })
        .chain(posts.array_windows().map(|[prev, post, next]| Page {
            item: post,
            id: post.id.clone(),
            prev: Some(base_url.join(&prev.id)),
            next: Some(base_url.join(&next.id)),
        }))
        .chain(std::iter::once(Page {
            item: &posts[posts.len() - 1],
            id: posts[posts.len() - 1].id.clone().into(),
            prev: Some(base_url.join(&posts[posts.len() - 2].id)),
            next: None,
        }))
        .collect(),
    }
}

fn index_pages<'a>(
    posts: &'a [PostSummary],
    page_size: usize,
    base_url: &'a Url,
) -> impl Iterator<Item = Page<Slice<'a, PostSummary>>> {
    let total_pages = match posts.len() % page_size {
        0 => posts.len() / page_size,
        _ => posts.len() / page_size + 1,
    };
    posts
        .chunks(page_size)
        .enumerate()
        .map(move |(page_number, posts)| Page {
            item: Slice::new(posts),
            id: format!("{}", page_number),
            prev: match page_number {
                0 => None,
                _ => Some(base_url.join((page_number - 1).to_string())),
            },
            next: match page_number + 1 < total_pages {
                false => None,
                true => Some(base_url.join((page_number + 1).to_string())),
            },
        })
}

fn build_indices<'a>(posts: &'a [Post<Tag>]) -> HashMap<String, Vec<&'a Post<Tag>>> {
    let mut m: HashMap<String, Vec<&'a Post<Tag>>> = HashMap::new();
    for post in posts {
        for tag in post.tags.iter() {
            match m.get_mut(&tag.tag) {
                None => {
                    m.insert(tag.tag.clone(), vec![post]);
                }
                Some(posts) => {
                    posts.push(post);
                }
            };
        }
    }
    // Include a "main index" consisting of all of the posts whose "tag" is the
    // empty string.
    m.insert(String::new(), posts.iter().collect());
    m
}
