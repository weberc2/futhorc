use crate::config::Config;
use crate::page::*;
use crate::post::*;
use crate::slice::*;
use crate::url::*;
use crate::write::*;
use anyhow::Result;
use gtmpl::Template;
use std::collections::HashMap;
use std::io::Read;
use std::path::Path;

pub fn build_site(config: &Config) -> Result<()> {
    // collect all posts
    let posts: Vec<Post<Tag>> = parse_posts(&config.posts_source_directory, config.threads)?
        .into_iter()
        .map(|p| p.convert_tags(&config.index_url))
        .collect();

    // Parse the template files.
    let index_template = parse_template(config.index_template.iter())?;
    let posts_template = parse_template(config.posts_template.iter())?;

    // render index pages
    render_indices(
        &build_indices(&posts),
        &config.index_url,
        &config.index_directory,
        &index_template,
        &config.posts_url,
        &config.home_page,
        config.index_page_size,
        config.threads,
    )?;

    // render post pages
    write_pages(
        post_pages(&posts, &config.posts_url).into_iter(),
        &config.posts_directory,
        &posts_template,
        &config.home_page,
        config.threads,
    )
}

fn render_indices(
    indices: &HashMap<String, Vec<&Post<Tag>>>,
    index_url: &Url,
    index_directory: &Path,
    index_template: &Template,
    posts_url: &Url,
    home_page: &Url,
    page_size: usize,
    threads: usize,
) -> anyhow::Result<()> {
    for (tag, index) in indices {
        render_index(
            index,
            tag,
            posts_url,
            index_url,
            index_directory,
            index_template,
            home_page,
            page_size,
            threads,
        )?;
    }
    Ok(())
}

fn render_index(
    index: &[&Post<Tag>],
    tag: &str,
    posts_url: &Url,
    index_url: &Url,
    index_directory: &Path,
    index_template: &Template,
    home_page: &Url,
    page_size: usize,
    threads: usize,
) -> anyhow::Result<()> {
    write_pages(
        index_pages(
            &index
                .into_iter()
                .map(|p| PostSummary::from((*p, posts_url)))
                .collect::<Vec<PostSummary>>(),
            page_size,
            index_url,
        ),
        &index_directory.join(&tag),
        index_template,
        home_page,
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
            next: Some(base_url.join(format!("{}.html", &posts[1].id))),
        })
        .chain(posts.array_windows().map(|[prev, post, next]| Page {
            item: post,
            id: post.id.clone(),
            prev: Some(base_url.join(format!("{}.html", &prev.id))),
            next: Some(base_url.join(format!("{}.html", &next.id))),
        }))
        .chain(std::iter::once(Page {
            item: &posts[posts.len() - 1],
            id: posts[posts.len() - 1].id.clone().into(),
            prev: Some(base_url.join(format!("{}.html", posts[posts.len() - 2].id))),
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
                _ => Some(base_url.join(format!("{}.html", page_number - 1))),
            },
            next: match page_number + 1 < total_pages {
                false => None,
                true => Some(base_url.join(format!("{}.html", page_number + 1))),
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

// Loads the template file contents, appends them to `base_template`, and
// parses the result into a template.
fn parse_template<'a, P: AsRef<Path>>(template_files: impl Iterator<Item = P>) -> Result<Template> {
    let mut contents = String::new();
    for template_file in template_files {
        use crate::util::open;
        open(template_file.as_ref(), "template")?.read_to_string(&mut contents)?;
        contents.push(' ');
    }
    let mut template = Template::default();
    match template.parse(&contents) {
        Err(e) => Err(anyhow::anyhow!(e)),
        Ok(_) => Ok(template),
    }
}
