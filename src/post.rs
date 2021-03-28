use crate::url::{Url, UrlBuf};
use anyhow::{anyhow, Result};
use pulldown_cmark::{html, Parser};
use serde::de::Error;
use serde::{Deserialize, Deserializer};
use serde_yaml;
use std::fs::{read_dir, File};
use std::io::prelude::*;
use std::path::Path;

#[derive(Clone, Debug)]
pub struct Tag {
    pub tag: String,
    pub url: UrlBuf,
}

impl std::str::FromStr for Tag {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Tag {
            tag: slug::slugify(s),
            url: UrlBuf::new(),
        })
    }
}

impl<'de> Deserialize<'de> for Tag {
    fn deserialize<D>(deserializer: D) -> Result<Tag, D::Error>
    where
        D: Deserializer<'de>,
    {
        String::deserialize(deserializer)?
            .parse::<Tag>()
            .map_err(|e| D::Error::custom(format!("{}", e)))
    }
}

impl Tag {
    pub fn deserialize_seq<'de, D>(deserializer: D) -> Result<Vec<Tag>, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct Wrapper(#[serde(deserialize_with = "Tag::deserialize")] Tag);

        let v = Vec::deserialize(deserializer)?;
        Ok(v.into_iter().map(|Wrapper(a)| a).collect())
    }
}

#[derive(Deserialize)]
pub struct Post<T> {
    #[serde(default)]
    pub id: String,

    #[serde(rename = "Title")]
    pub title: String,

    #[serde(rename = "Date")]
    pub date: String,

    #[serde(default)]
    pub body: String,

    #[serde(default, rename = "Tags")]
    pub tags: Vec<T>,
}

impl Post<String> {
    pub fn convert_tags(&self, tags_base_url: &Url) -> Post<Tag> {
        Post {
            id: self.id.clone(),
            title: self.title.clone(),
            date: self.date.clone(),
            body: self.body.clone(),
            tags: self
                .tags
                .iter()
                .map(|t| Tag {
                    tag: t.clone(),
                    url: tags_base_url.join(format!("{}/0.html", t)),
                })
                .collect(),
        }
    }

    pub fn from_str(id: &str, input: &str) -> anyhow::Result<Self> {
        fn frontmatter_indices(input: &str) -> anyhow::Result<(usize, usize, usize)> {
            const FENCE: &str = "---";
            if !input.starts_with(FENCE) {
                return Err(anyhow!("Post must begin with `---`"));
            }
            match input[FENCE.len()..].find("---") {
                None => Err(anyhow!("Missing closing `---`")),
                Some(offset) => Ok((
                    FENCE.len(),                        // yaml_start
                    FENCE.len() + offset,               // yaml_stop
                    FENCE.len() + offset + FENCE.len(), // body_start
                )),
            }
        }

        let (yaml_start, yaml_stop, body_start) = frontmatter_indices(input)?;
        let mut post: Post<String> = serde_yaml::from_str(&input[yaml_start..yaml_stop])?;
        post.id = id.to_owned();
        html::push_html(&mut post.body, Parser::new(&input[body_start..]));
        Ok(post)
    }
}

impl<T> Post<T> {
    pub fn summary(&self) -> (&str, bool) {
        const FOLD_TAG: &str = "<!-- more -->";
        match self.body.find(FOLD_TAG) {
            Some(i) => (&self.body[..i], true),
            None => (&self.body, false),
        }
    }
}

pub fn parse_posts_parallel(dir: &Path, threads: usize) -> Result<Vec<Post<String>>> {
    use crossbeam_channel::unbounded;
    use std::path::PathBuf;
    use std::thread;

    let (tx, rx) = unbounded::<(String, PathBuf)>();
    let mut threads = Vec::with_capacity(threads);

    for _ in 0..threads.capacity() {
        let rx = rx.clone();
        threads.push(thread::spawn(move || -> Result<Vec<Post<String>>> {
            let mut v: Vec<Post<String>> = Vec::new();
            for (file_name, full_path) in rx {
                v.push(process_entry(&file_name, &full_path)?);
            }
            Ok(v)
        }))
    }

    for result in read_dir(dir)? {
        let entry = result?;
        let os_file_name = entry.file_name();
        let file_name = os_file_name.to_string_lossy();
        if file_name.ends_with(MARKDOWN_EXTENSION) {
            tx.send((file_name.to_string(), entry.path()))?;
        }
    }
    drop(tx);

    let mut posts: Vec<Post<String>> = Vec::new();
    for thread in threads {
        posts.extend(thread.join().unwrap()?);
    }
    posts.sort_by(|a, b| b.date.cmp(&a.date));
    Ok(posts)
}

const MARKDOWN_EXTENSION: &str = ".md";

fn process_entry(file_name: &str, full_path: &Path) -> Result<Post<String>> {
    let base_name = file_name.trim_end_matches(MARKDOWN_EXTENSION);
    let mut contents = String::new();
    File::open(full_path)?.read_to_string(&mut contents)?;
    Post::from_str(base_name, &contents)
}

pub fn parse_posts(dir: &Path, threads: usize) -> Result<Vec<Post<String>>> {
    if threads < 2 {
        parse_posts_singlethreaded(dir)
    } else {
        parse_posts_parallel(dir, threads)
    }
}

// Walks `dir` and returns a vector of posts ordered by date.
pub fn parse_posts_singlethreaded(dir: &Path) -> Result<Vec<Post<String>>> {
    let mut posts: Vec<Post<String>> = Vec::new();

    for result in read_dir(dir)? {
        let entry = result?;
        let os_file_name = entry.file_name();
        let file_name = os_file_name.to_string_lossy();
        if file_name.ends_with(MARKDOWN_EXTENSION) {
            posts.push(process_entry(&file_name, &entry.path())?);
        }
    }

    posts.sort_by(|a, b| b.date.cmp(&a.date));

    Ok(posts)
}

#[derive(Clone)]
pub struct PostSummary {
    pub id: String,
    pub url: UrlBuf,
    pub title: String,
    pub date: String,
    pub summary: String,
    pub summarized: bool,
    pub tags: Vec<Tag>,
}

impl From<(&Post<Tag>, &Url)> for PostSummary {
    fn from(tuple: (&Post<Tag>, &Url)) -> PostSummary {
        let (p, base_url) = tuple;
        let (summary, summarized) = p.summary();
        PostSummary {
            id: p.id.clone(),
            url: base_url.join(&format!("{}.html", p.id)),
            title: p.title.clone(),
            date: p.date.clone(),
            summary: summary.to_owned(),
            summarized: summarized,
            tags: p.tags.clone(),
        }
    }
}
