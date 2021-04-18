use crate::url::*;
use serde::de::{Deserialize, Deserializer};
use std::hash::{Hash, Hasher};

#[derive(Clone, Debug)]
pub struct Tag {
    pub tag: String,
    pub url: UrlBuf,
}

impl<'de> Deserialize<'de> for Tag {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Tag, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(Tag {
            tag: slug::slugify(&String::deserialize(deserializer)?),
            url: UrlBuf::new(),
        })
    }
}

impl Hash for Tag {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.tag.hash(state)
    }
}

impl PartialEq for Tag {
    fn eq(&self, other: &Self) -> bool {
        self.tag == other.tag
    }
}
impl Eq for Tag {}
