//! Defines the [`Tag`] type, which represents a [`crate::post::Post`] tag.

use gtmpl::Value;
use std::hash::{Hash, Hasher};
use url::Url;

/// Represents a [`crate::post::Post`] tag. On parsing a post from YAML, only
/// the `name` field is parsed while the `url` field is left empty. The URL
/// field must be filled in later based on the `index_base_url` and the tag
/// name.
#[derive(Clone, Debug)]
pub struct Tag {
    /// The tag's name. This should be slugified so e.g., `macOS` and `MacOS`
    /// resolve to the same value, and also so the field can be dropped into a
    /// [`Url`].
    pub name: String,

    /// The URL for the tag's first index page. Given an `index_base_url`,
    /// this should look something like
    /// `{index_base_url}/{tag_name}/index.html`.
    pub url: Url,
}

impl Hash for Tag {
    /// Implements [`Hash`] for [`Tag`] by delegating directly to the `name`
    /// field.
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.name.hash(state)
    }
}

impl PartialEq for Tag {
    /// Implements [`PartialEq`] and [`Eq`] for [`Tag`] by delegating directly
    /// to the `name` field.
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
    }
}
impl Eq for Tag {}

impl From<&Tag> for Value {
    /// Converts [`Tag`]s into [`Value`]s for templating.
    fn from(t: &Tag) -> Value {
        use std::collections::HashMap;
        let mut m: HashMap<String, Value> = HashMap::new();
        m.insert("tag".to_owned(), (&t.name).into());
        m.insert("url".to_owned(), Value::String(t.url.to_string()));
        Value::Object(m)
    }
}
