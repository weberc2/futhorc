use url::{ParseError, Url};

const MARKDOWN_EXTENSION: &str = ".md";
const HTML_EXTENSION: &str = ".html";

pub struct Converter<'a> {
    posts_root: &'a Url,
    base: Url,
}

impl<'a> Converter<'a> {
    /// [`posts_url`] is the URL prefix for posts. [`base`] is the relative
    /// path from [`posts_url`] from which target URLs will be converted.
    pub fn new(posts_root: &'a Url, base: &str) -> Result<Converter<'a>> {
        Ok(Converter {
            posts_root,
            base: posts_root.join(base)?,
        })
    }

    fn convert_absolute<'b>(&self, absolute: Url) -> Result<Url> {
        if let Some(relative) = self.posts_root.make_relative(&absolute) {
            if !relative.starts_with("../")
                && relative.ends_with(MARKDOWN_EXTENSION)
            {
                let abs = absolute.to_string();
                return Ok(Url::parse(&format!(
                    "{}{}",
                    &abs[..abs.len() - MARKDOWN_EXTENSION.len()],
                    HTML_EXTENSION,
                ))
                .unwrap()); // this should never fail
            }
        }
        Ok(absolute)
    }

    fn convert_unknown(&self, url: &str) -> Result<Url> {
        match Url::parse(url) {
            Ok(absolute) => self.convert_absolute(absolute),
            Err(ParseError::RelativeUrlWithoutBase) => {
                self.convert_absolute(self.base.join(url)?)
            }
            Err(e) => Err(e),
        }
    }

    pub fn convert(&self, url: &str) -> Result<String> {
        Ok(self.convert_unknown(url)?.to_string())
    }
}

type Result<T> = std::result::Result<T, ParseError>;

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_convert_relative_post() -> Result<()> {
        fixture_basic("https://example.org/posts/relative.html", "relative.md")
    }

    #[test]
    fn test_convert_relative_post_leading_dotslash() -> Result<()> {
        fixture_basic(
            "https://example.org/posts/relative.html",
            "./relative.md",
        )
    }

    #[test]
    fn test_convert_relative_post_redundancies() -> Result<()> {
        fixture_basic(
            "https://example.org/posts/relative.html",
            "../posts/relative.md",
        )
    }

    #[test]
    fn test_convert_relative_asset() -> Result<()> {
        fixture_basic("https://example.org/posts/relative.jpg", "relative.jpg")
    }

    #[test]
    fn test_convert_relative_asset_leading_dotslash() -> Result<()> {
        fixture_basic(
            "https://example.org/posts/relative.jpg",
            "./relative.jpg",
        )
    }

    #[test]
    fn test_convert_relative_asset_redundancies() -> Result<()> {
        fixture_basic(
            "https://example.org/posts/relative.jpg",
            "../posts/relative.jpg",
        )
    }

    #[test]
    fn test_convert_absolute_post() -> Result<()> {
        fixture_basic(
            "https://example.org/posts/absolute.html",
            "https://example.org/posts/absolute.md",
        )
    }

    #[test]
    fn test_convert_absolute_asset() -> Result<()> {
        fixture_basic(
            "https://example.org/posts/absolute.jpg",
            "https://example.org/posts/absolute.jpg",
        )
    }

    #[test]
    fn test_convert_absolute_asset_redundancies() -> Result<()> {
        fixture_basic(
            "https://example.org/posts/absolute.jpg",
            "https://example.org/posts/../posts/absolute.jpg",
        )
    }

    #[test]
    fn test_convert_remote_markdown() -> Result<()> {
        fixture_basic(
            "https://remote.org/absolute.md",
            "https://remote.org/absolute.md",
        )
    }

    #[test]
    fn test_convert_remote_markdown_redundancies() -> Result<()> {
        fixture_basic(
            "https://remote.org/posts/absolute.md",
            "https://remote.org/posts/../posts/absolute.md",
        )
    }

    fn fixture_basic(wanted: &str, target: &str) -> Result<()> {
        assert_eq!(
            wanted,
            Converter::new(
                &Url::parse("https://example.org/posts/")?,
                "index.html"
            )?
            .convert(target)?,
        );
        Ok(())
    }
}
