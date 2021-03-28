use serde::{Deserialize, Deserializer};
use std::ops::Deref;

pub struct Url(str);

impl Url {
    pub fn new<S: AsRef<str> + ?Sized>(url: &S) -> &Url {
        unsafe { &*(url.as_ref().trim_end_matches('/') as *const str as *const Url) }
    }

    pub fn join<D: std::fmt::Display>(&self, rhs: D) -> UrlBuf {
        UrlBuf(format!("{}/{}", &self.0, rhs))
    }

    pub fn to_owned(&self) -> UrlBuf {
        UrlBuf(self.0.to_owned())
    }

    #[inline]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl AsRef<str> for Url {
    #[inline]
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl From<&Url> for String {
    fn from(url: &Url) -> String {
        url.0.into()
    }
}

impl std::fmt::Display for Url {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", &self.0)
    }
}

#[derive(Clone)]
pub struct UrlBuf(String);

impl UrlBuf {
    pub fn new() -> UrlBuf {
        UrlBuf(String::new())
    }

    pub fn into_string(self) -> String {
        self.0
    }
}

impl From<UrlBuf> for String {
    fn from(url_buf: UrlBuf) -> String {
        url_buf.0
    }
}

impl Deref for UrlBuf {
    type Target = Url;
    #[inline]
    fn deref(&self) -> &Url {
        Url::new(&self.0)
    }
}

impl std::borrow::Borrow<Url> for UrlBuf {
    #[inline]
    fn borrow(&self) -> &Url {
        self.deref()
    }
}

impl AsRef<Url> for UrlBuf {
    #[inline]
    fn as_ref(&self) -> &Url {
        self
    }
}

impl<'de> Deserialize<'de> for UrlBuf {
    fn deserialize<D>(d: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(UrlBuf(
            String::deserialize(d)?.trim_end_matches('/').to_owned(),
        ))
    }
}
