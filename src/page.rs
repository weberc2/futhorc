pub struct Page<T> {
    pub item: T,
    pub id: String,
    pub prev: Option<String>,
    pub next: Option<String>,
}