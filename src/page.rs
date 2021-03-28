use crate::url::UrlBuf;

pub struct Page<T> {
    pub item: T,
    pub id: String,
    pub prev: Option<UrlBuf>,
    pub next: Option<UrlBuf>,
}
