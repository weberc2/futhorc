//! The library code for the `futhorc` static site generator. The architecture
//! can be generally broken down into two distinct steps:
//!
//! 1. Parsing posts from source files on disk ([`crate::post`])
//! 2. Converting the posts into output files on disk ([`crate::write`])
//!
//! Of the two, the second step is the more involved. It is itself composed of
//! three distinct sub-steps:
//!
//! 1. Building post pages
//! 2. Building index pages
//! 3. Rendering all pages to disk
//!
//! Again here the second sub-step is the more involved, because we need to
//! create groups of index pages for each tag and another group for the empty tag
//! which corresponds to all posts. A group of index pages is referred to as an
//! "index", and each index is paginated--converted into groups of pages based on
//! a configurable number of posts per index page.
//!
//! The third substep is pretty straight-forward: for each page, apply the
//! template (either the post template or the index template) and write the
//! result to disk.

#![cfg_attr(feature = "fail-on-warnings", deny(warnings))]
#![feature(map_into_keys_values)]

pub mod build;
pub mod config;
pub mod feed;
pub mod htmlrenderer;
pub mod post;
pub mod tag;
pub mod url;
pub mod write;
