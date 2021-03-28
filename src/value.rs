use crate::page::*;
use crate::post::*;
use crate::slice::Slice;
use crate::url::{Url, UrlBuf};
use gtmpl_value::Value;
use std::collections::HashMap;

impl<T> From<Slice<'_, T>> for Value
where
    for<'b> &'b T: Into<Value>,
{
    fn from(s: Slice<T>) -> Value {
        Value::Array(s.iter().map(Value::from).collect())
    }
}

impl From<&PostSummary> for Value {
    fn from(p: &PostSummary) -> Value {
        let mut m: HashMap<String, Value> = HashMap::new();
        m.insert("id".to_owned(), (&p.id).into());
        m.insert("url".to_owned(), (&p.url).into());
        m.insert("title".to_owned(), (&p.title).into());
        m.insert("date".to_owned(), (&p.date).into());
        m.insert("summary".to_owned(), (&p.summary).into());
        m.insert("summarized".to_owned(), Value::Bool(p.summarized));
        m.insert("tags".to_owned(), (&p.tags).as_slice().into());
        Value::Object(m)
    }
}

impl From<Tag> for Value {
    fn from(t: Tag) -> Value {
        let mut m: HashMap<String, Value> = HashMap::new();
        m.insert("tag".to_owned(), (&t.tag).into());
        m.insert("url".to_owned(), (&t.url).into());
        Value::Object(m)
    }
}

impl From<&Post<Tag>> for Value {
    fn from(p: &Post<Tag>) -> Value {
        let mut m: HashMap<String, Value> = HashMap::new();
        m.insert("id".to_owned(), (&p.id).into());
        m.insert("title".to_owned(), (&p.title).into());
        m.insert("date".to_owned(), (&p.date).into());
        m.insert("body".to_owned(), (&p.body).into());
        m.insert("tags".to_owned(), (&p.tags).as_slice().into());
        Value::Object(m)
    }
}

impl<T> From<Page<T>> for Value
where
    T: Into<Value>,
{
    fn from(p: Page<T>) -> Value {
        let mut m: HashMap<String, Value> = HashMap::new();
        m.insert("item".to_owned(), p.item.into());
        m.insert("id".to_owned(), p.id.into());
        m.insert(
            "prev".to_owned(),
            match p.prev {
                None => Value::NoValue,
                Some(id) => id.into(),
            },
        );
        m.insert("next".to_owned(), p.next.into());
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
        Value::from(url.as_ref())
    }
}
