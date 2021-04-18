use crate::tag::*;
use crate::url::{Url, UrlBuf};
use gtmpl_value::Value;
use std::collections::HashMap;

impl From<&Tag> for Value {
    fn from(t: &Tag) -> Value {
        let mut m: HashMap<String, Value> = HashMap::new();
        m.insert("tag".to_owned(), (&t.name).into());
        m.insert("url".to_owned(), (&t.url).into());
        Value::Object(m)
    }
}

impl From<&Url> for Value {
    fn from(url: &Url) -> Value {
        Value::String(url.to_string())
    }
}

impl From<UrlBuf> for Value {
    fn from(url: UrlBuf) -> Value {
        Value::from(&url)
    }
}

impl From<&UrlBuf> for Value {
    fn from(url: &UrlBuf) -> Value {
        let url: &Url = url;
        Value::from(url)
    }
}
