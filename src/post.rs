//! Defines the [`Post`], [`Parser`], and [`Error`] types. Also defines the
//! logic for parsing posts from the file system into memory. See the
//! [`Post::to_value`] and [`Post::summarize`] for details on how posts are
//! converted into template values.

use crate::markdown;
use crate::tag::Tag;
use gtmpl::Value;
use serde::Deserialize;
use std::collections::HashSet;
use std::fmt;
use std::fs::{read_dir, File};
use std::path::{Path, PathBuf};
use url::Url;

/// Represents a blog post.
#[derive(Clone)]
pub struct Post {
    /// The output path where the final post file will be rendered.
    pub file_path: PathBuf,

    /// The address for the rendered post.
    pub url: Url,

    /// The title of the post.
    pub title: String,

    /// The date of the post.
    pub date: String,

    /// The body of the post.
    pub body: String,

    /// The tags associated with the post.
    pub tags: HashSet<Tag>,
}

impl Post {
    /// Converts a [`Post`] into a template-renderable [`Value`], representing
    /// a full post (as opposed to [`Post::summarize`] which represents a
    /// post summary). The resulting [`Value`] has fields:
    ///
    /// * `url`: The url of the post
    /// * `title`: The title of the post
    /// * `date`: The published date of the post
    /// * `body`: The post body
    /// * `tags`: A list of tags associated with the post
    pub fn to_value(&self) -> Value {
        use std::collections::HashMap;
        let mut m = HashMap::new();
        m.insert("url".to_owned(), Value::String(self.url.to_string()));
        m.insert("title".to_owned(), Value::String(self.title.clone()));
        m.insert("date".to_owned(), Value::String(self.date.clone()));
        m.insert("body".to_owned(), Value::String(self.body.clone()));
        m.insert(
            "tags".to_owned(),
            Value::Array(self.tags.iter().map(Value::from).collect()),
        );
        Value::Object(m)
    }

    /// Returns the full post body unless a `<!-- more -->` tag was found, in
    /// which case it returns the text up to that tag (the "summary" text). It
    /// also returns a boolean value indicating whether or not the tag was
    /// found.
    pub fn summary(&self) -> (&str, bool) {
        match self.body.find("<!-- more -->") {
            None => (self.body.as_str(), false),
            Some(idx) => (&self.body[..idx], true),
        }
    }

    /// Converts a [`Post`] into a template-renderable [`Value`] representing a
    /// post summary. The resulting [`Value`] has fields:
    ///
    /// * `url`: The url of the post
    /// * `title`: The title of the post
    /// * `date`: The published date of the post
    /// * `summary`: The post summary if there is a `<!-- more -->` tag or else
    ///   the full post body
    /// * `summarized`: A boolean value representing whether or not a `<!--
    ///   more -->` tag was found and thus the post was truncated.
    /// * `tags`: A list of tags associated with the post
    pub fn summarize(&self) -> Value {
        use std::collections::HashMap;
        let (summary, summarized) = self.summary();

        let mut m = HashMap::new();
        m.insert("url".to_owned(), Value::String(self.url.to_string()));
        m.insert("title".to_owned(), Value::String(self.title.clone()));
        m.insert("date".to_owned(), Value::String(self.date.clone()));
        m.insert("summary".to_owned(), Value::String(summary.to_string()));
        m.insert("summarized".to_owned(), Value::Bool(summarized));
        m.insert(
            "tags".to_owned(),
            Value::Array(self.tags.iter().map(Value::from).collect()),
        );
        Value::Object(m)
    }
}
