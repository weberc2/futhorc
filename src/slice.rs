// Slice exists because we can't override `From<&[T]> for Value`. We want to
// override it because the standard variant requires that `T` implements
// `Clone`, which implies an unnecessary copy. Probably not a big deal, but
// I'm stubborn.

pub struct Slice<'a, T>(&'a [T]);

impl<'a, T> From<Slice<'a, T>> for &'a [T] {
    fn from(s: Slice<'a, T>) -> &'a [T] { s.0 }
}

impl<'a, T> Slice<'a, T> {
    pub fn new(items: &'a [T]) -> Self { Self(items) }

    pub fn iter(&self) -> impl Iterator<Item = &'a T> { self.0.iter() }
}