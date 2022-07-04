//! Defines [`Url`] and [`UrlBuf`] types which are analogous to the [`str`] and
//! [`String`] or [`std::path::Path`] and [`std::path::PathBuf`] pairs. These
//! are effectively newtypes for [`str`] and [`String`].

use gtmpl::Value;
use serde::{Deserialize, Deserializer};
use std::ops::Deref;

/// A newtype for URL strings, analogous to [`str`].
pub struct Url(str);

impl Url {
    /// Construct a new `&Url` from any [`AsRef<str>`].
    #[inline]
    pub fn new<S: AsRef<str> + ?Sized>(url: &S) -> &Url {
        unsafe {
            &*(url.as_ref().trim_end_matches('/') as *const str as *const Url)
        }
    }

    /// Join any [`AsRef<str>`] to the `&Url`.
    #[inline]
    pub fn join<S: AsRef<str>>(&self, rhs: S) -> UrlBuf {
        let mut buf = self.to_url_buf();
        buf.0.push('/');
        buf.0.push_str(rhs.as_ref());
        buf
    }

    /// Create a [`UrlBuf`] from the current `&Url`.
    #[inline]
    pub fn to_url_buf(&self) -> UrlBuf {
        UrlBuf::from(self.0.to_string())
    }

    /// Return the [`str`] representation of the current `&Url`.
    #[inline]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl ToOwned for Url {
    type Owned = UrlBuf;
    /// Returns the owned [`UrlBuf`] corresponding to the current `&Url`.
    #[inline]
    fn to_owned(&self) -> UrlBuf {
        UrlBuf(self.0.to_owned())
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

/// A newtype for URL strings, analogous to [`String`].
#[derive(Clone, Debug)]
pub struct UrlBuf(String);

impl UrlBuf {
    /// Constructs a new [`UrlBuf`]; corresponds to [`String::new`].
    #[inline]
    pub fn new() -> UrlBuf {
        UrlBuf(String::new())
    }

    /// Consumes the current [`UrlBuf`] and returns the [`String`]
    /// representation.
    #[inline]
    pub fn into_string(self) -> String {
        self.0
    }
}

impl Default for UrlBuf {
    /// Returns a default [`UrlBuf`]. This corresponds to [`String::default`].
    #[inline]
    fn default() -> UrlBuf {
        UrlBuf(String::default())
    }
}

impl std::fmt::Display for UrlBuf {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let url: &Url = self;
        url.fmt(f)
    }
}

impl From<UrlBuf> for String {
    #[inline]
    fn from(url_buf: UrlBuf) -> String {
        url_buf.0
    }
}

impl From<String> for UrlBuf {
    #[inline]
    fn from(s: String) -> UrlBuf {
        UrlBuf(s)
    }
}

impl From<&str> for UrlBuf {
    #[inline]
    fn from(s: &str) -> UrlBuf {
        UrlBuf(s.to_owned())
    }
}

impl AsRef<str> for UrlBuf {
    #[inline]
    fn as_ref(&self) -> &str {
        let url: &Url = self;
        url.as_ref()
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
