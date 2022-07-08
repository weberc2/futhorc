use crate::post::{HTML_EXTENSION, MARKDOWN_EXTENSION};
use std::borrow::Cow;
use url::{ParseError, Url};

/// `posts_url` contains the URL to the `posts` directory, e.g., `https://example.org/posts/`.
/// `base` must be a path to a file inside of `posts_url`. `url` must be either an absolute path
/// (e.g., `https://example.org/posts/foo.md`) or a path relative to `base` (e.g., `foo.md`).
pub fn convert<'a>(
    posts_url: &Url,
    base: &str,
    url: &'a str,
) -> Result<String, ParseError> {
    println!("posts_url: {}", posts_url);
    println!("base:      {}", base);
    println!("url:       {}", url);
    // `base_in_url` is the url referencing the `url`
    let base_in_url = posts_url.join(base)?;

    // `absolute` is the absolute URL for `url`.
    let absolute = match Url::parse(url) {
        Ok(u) => u,
        Err(ParseError::RelativeUrlWithoutBase) => base_in_url.join(url)?,
        Err(e) => return Err(e),
    };

    // make the absolute URL relative to the posts directory URL as required
    // by md2html (`base` could be in a post-bundle directory, and if we passed
    // md2html a URL that was base-relative, then it may not properly detect
    // post-bundles).
    Ok(match posts_url.make_relative(&absolute) {
        Some(posts_url_rel) => {
            // Should never fail; we should always be able to join a
            // post-relative URL to the posts directory URL.
            posts_url.join(&md2html(&posts_url_rel)).unwrap().to_string()
        }

        // if we get here, `absolute` cannot be made relative to `posts_url`
        // and thus it is probably on some other host. We could probably return
        // `url`, but by returning `absolute`, we can guarantee that the URL is
        // normalized (e.g., if `url` is `http://foo.com/./bar`, we still want
        // to return `http://foo.com/bar`).
        None => absolute.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestCase {
        posts_url: Url,
        base: &'static str,
        url: &'static str,
        wanted: &'static str,
    }

    fn convert_test(test_case: &TestCase) -> Result<(), ParseError> {
        let result =
            convert(&test_case.posts_url, &test_case.base, test_case.url)?;
        assert_eq!(
            test_case.wanted, result,
            "wanted \"{}\"; found \"{}\"",
            test_case.wanted, result
        );
        Ok(())
    }

    #[test]
    fn test_convert_absolute_url_to_unbundled_post() -> Result<(), ParseError>
    {
        convert_test(&TestCase {
            posts_url: Url::parse("https://example.org/posts/")?,
            base: "base.html",
            url: "https://example.org/posts/post.md",
            wanted: "https://example.org/posts/post.html",
        })
    }

    #[test]
    fn test_convert_relative_url_to_unbundled_post() -> Result<(), ParseError>
    {
        convert_test(&TestCase {
            posts_url: Url::parse("https://example.org/posts/")?,
            base: "base.html",
            url: "post.md",
            wanted: "https://example.org/posts/post.html",
        })
    }

    #[test]
    fn test_convert_relative_url_to_unbundled_post_leading_dotslash(
    ) -> Result<(), ParseError> {
        convert_test(&TestCase {
            posts_url: Url::parse("https://example.org/posts/")?,
            base: "base.html",
            url: "./post.md",
            wanted: "https://example.org/posts/post.html",
        })
    }

    #[test]
    fn test_convert_absolute_url_to_bundled_post() -> Result<(), ParseError> {
        convert_test(&TestCase {
            posts_url: Url::parse("https://example.org/posts/")?,
            base: ".",
            url: "https://example.org/posts/post/index.md",
            wanted: "https://example.org/posts/post.html",
        })
    }

    #[test]
    fn test_convert_relative_url_to_bundled_post() -> Result<(), ParseError> {
        convert_test(&TestCase {
            posts_url: Url::parse("https://example.org/posts/")?,
            base: "base.html",
            url: "post/index.md",
            wanted: "https://example.org/posts/post.html",
        })
    }

    #[test]
    fn test_convert_relative_url_to_bundled_post_leading_dotslash(
    ) -> Result<(), ParseError> {
        convert_test(&TestCase {
            posts_url: Url::parse("https://example.org/posts/")?,
            base: "base.html",
            url: "./post/index.md",
            wanted: "https://example.org/posts/post.html",
        })
    }

    #[test]
    fn test_convert_relative_url_to_unbundled_post_from_bundle(
    ) -> Result<(), ParseError> {
        convert_test(&TestCase {
            posts_url: Url::parse("https://example.org/posts/")?,
            base: "post/index.md",
            url: "../foo.md",
            wanted: "https://example.org/posts/foo.html",
        })
    }

    #[test]
    fn test_convert_relative_url_to_bundled_post_from_bundle(
    ) -> Result<(), ParseError> {
        convert_test(&TestCase {
            posts_url: Url::parse("https://example.org/posts/")?,
            base: "post/index.md",
            url: "foo.jpg",
            wanted: "https://example.org/posts/post/foo.jpg",
        })
    }

    #[test]
    fn test_convert_indirect_relative_url_to_bundle_from_bundle(
    ) -> Result<(), ParseError> {
        convert_test(&TestCase {
            posts_url: Url::parse("https://example.org/posts/")?,
            base: "post/index.md",
            url: "../post/foo.jpg",
            wanted: "https://example.org/posts/post/foo.jpg",
        })
    }

    #[test]
    fn test_convert_from_bundle_to_same_bundle() -> Result<(), ParseError> {
        convert_test(&TestCase {
            posts_url: Url::parse("https://example.org/posts/")?,
            base: "post/",
            url: "index.md",
            wanted: "https://example.org/posts/post.html",
        })
    }

    #[test]
    fn test_convert_from_bundle_to_same_bundle_indirect(
    ) -> Result<(), ParseError> {
        convert_test(&TestCase {
            posts_url: Url::parse("https://example.org/posts/")?,
            base: "post/",
            url: "../post/index.md",
            wanted: "https://example.org/posts/post.html",
        })
    }
}

// `relative` must be a normalized URL path relative to the output posts URL.
// Returns `None` if `relative` isn't a markdown file at all.
fn md2html(relative: &str) -> Cow<str> {
    let path = relative.trim_start_matches('/');
    if let Some(out) = replace_suffix(path, "/index.md", HTML_EXTENSION) {
        return Cow::Owned(out);
    }
    if let Some(out) = replace_suffix(path, MARKDOWN_EXTENSION, HTML_EXTENSION)
    {
        return Cow::Owned(out);
    }
    return Cow::Borrowed(relative);
}

fn replace_suffix(input: &str, before: &str, after: &str) -> Option<String> {
    if input.ends_with(before) {
        let mut out = String::from(&input[..input.len() - before.len()]);
        out.push_str(after);
        return Some(out);
    }
    return None;
}
