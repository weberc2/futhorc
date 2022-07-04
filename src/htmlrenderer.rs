//! Implements a custom [`push_html`] to support footnotes in summaries.
//! [`pulldown_cmark::html::push_html`] assumes that the footnote definition is
//! on the same page as the footnote reference, which is true for post pages,
//! but not for the index pages (in cases where the footnote reference appears
//! above the fold in the post summary, but the footnote definition is at the
//! bottom of the post page).

use pulldown_cmark::escape::{escape_href, escape_html, StrWrite};
use pulldown_cmark::{Alignment, CodeBlockKind, CowStr, Event, LinkType, Tag};
use std::fmt::{self, Display};
use std::io;

struct Adaptor<'a, T> {
    formatter: &'a mut T,
    result: fmt::Result,
}

impl<T> Adaptor<'_, T> {
    fn handle_result(&mut self, result: fmt::Result) -> io::Result<()> {
        match result {
            Ok(_) => Ok(()),
            Err(e) => {
                self.result = result;
                Err(io::Error::new(io::ErrorKind::Other, e))
            }
        }
    }
}

impl<T: fmt::Write> StrWrite for Adaptor<'_, T> {
    fn write_str(&mut self, s: &str) -> io::Result<()> {
        let result = self.formatter.write_str(s);
        self.handle_result(result)
    }

    fn write_fmt(&mut self, args: fmt::Arguments) -> io::Result<()> {
        let result = self.formatter.write_fmt(args);
        self.handle_result(result)
    }
}

struct EscapeHref<'a>(CowStr<'a>);

impl<'a> Display for EscapeHref<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut adaptor = Adaptor {
            formatter: f,
            result: Ok(()),
        };
        let _ = escape_href(&mut adaptor, &self.0);
        adaptor.result
    }
}

struct EscapeHtml<'a>(CowStr<'a>);

impl<'a> Display for EscapeHtml<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut adaptor = Adaptor {
            formatter: f,
            result: Ok(()),
        };

        let _ = escape_html(&mut adaptor, &self.0);
        adaptor.result
    }
}

enum TableState {
    Head,
    Body,
}

/// Renders markdown [`Event`]s into HTML. This is largely modeled after
/// [`pulldown_cmark`]'s private [`HtmlWriter`
/// struct](https://github.com/raphlinus/pulldown-cmark/blob/bf0a1a4938dbd2ec41c3add069b3d361d11731f4/src/html.rs#L36-L50).
struct HtmlRenderer {
    table_alignments: Vec<Alignment>,
    table_state: TableState,
    table_cell_index: usize,

    /// The prefix to prepend onto footnote links.
    footnote_prefix: String,
}

impl<'a> HtmlRenderer {
    fn on_event<W: StrWrite>(
        &mut self,
        w: &mut W,
        event: Event<'a>,
    ) -> io::Result<()> {
        match event {
            Event::Start(tag) => self.on_start(w, tag),
            Event::End(tag) => self.on_end(w, tag),
            Event::Code(code) => self.on_code(w, code),
            Event::FootnoteReference(name) => write!(
                w,
                r#"<sup class="footnote-reference"><a href="{}#{}">{}</a></sup>"#,
                EscapeHtml(CowStr::from(self.footnote_prefix.as_str())),
                name,
                name,
            ),
            Event::HardBreak => self.on_hard_break(w),
            Event::Html(html) => self.on_html(w, html),
            Event::Rule => self.on_rule(w),
            Event::SoftBreak => self.on_soft_break(w),
            Event::TaskListMarker(checked) => {
                self.on_task_list_marker(w, checked)
            }
            Event::Text(text) => self.on_text(w, text),
        }
    }
}

impl<'a> HtmlRenderer {
    fn new() -> Self {
        HtmlRenderer {
            table_alignments: Vec::default(),
            table_state: TableState::Head,
            table_cell_index: usize::default(),
            footnote_prefix: String::default(),
        }
    }

    fn with_footnote_prefix(footnote_prefix: &str) -> Self {
        let mut renderer = Self::new();
        renderer.footnote_prefix = footnote_prefix.to_owned();
        renderer
    }

    fn on_start<W: StrWrite>(
        &mut self,
        w: &mut W,
        tag: Tag<'a>,
    ) -> io::Result<()> {
        match tag {
            Tag::BlockQuote => write!(w, "<blockquote>"),
            Tag::CodeBlock(kind) => match kind {
                CodeBlockKind::Fenced(info) => match info.split(' ').next() {
                    None => panic!(
                        "There must be at least one result from split()"
                    ),
                    Some(lang) => match lang.is_empty() {
                        true => write!(w, "<pre><code>"),
                        false => write!(
                            w,
                            r#"<pre><code class="language-{}">"#,
                            lang
                        ),
                    },
                },
                CodeBlockKind::Indented => w.write_str("<pre><code>"),
            },
            Tag::Emphasis => w.write_str("<em>"),
            Tag::FootnoteDefinition(name) => {
                let name = EscapeHtml(name);
                write!(
                    w,
                    r#"<div class="footnote-definition" id="{}">{}. &nbsp;"#,
                    &name, &name,
                )
            }
            Tag::Heading(size) => write!(w, "<h{}>", size),
            Tag::Image(_link_type, dest, title) => write!(
                w,
                // TODO: Handle alt text
                r#"<img src="{}" alt="" title="{}">"#,
                EscapeHref(dest),
                EscapeHtml(title),
            ),
            Tag::Item => w.write_str("<li>"),
            Tag::Link(LinkType::Email, dest, title) => write!(
                w,
                r#"<a href="mailto:{}" title="{}">"#,
                EscapeHref(dest),
                EscapeHtml(title),
            ),
            Tag::Link(_link_type, dest, title) => write!(
                w,
                r#"<a href="{}" title="{}">"#,
                EscapeHref(dest),
                EscapeHtml(title),
            ),
            Tag::List(None) => w.write_str("<ul>"),
            Tag::List(Some(1)) => w.write_str("<ol>"),
            Tag::List(Some(start)) => write!(w, r#"<ol start="{}">"#, start),
            Tag::Paragraph => write!(w, "<p>"),
            Tag::Strikethrough => w.write_str("<del>"),
            Tag::Strong => w.write_str("<strong>"),
            Tag::Table(alignments) => {
                self.table_alignments = alignments;
                w.write_str("<table>")
            }
            Tag::TableHead => {
                self.table_state = TableState::Head;
                self.table_cell_index = 0;
                w.write_str("<thead><tr>")
            }
            Tag::TableRow => {
                self.table_cell_index = 0;
                w.write_str("<tr>")
            }
            Tag::TableCell => write!(
                w,
                "<{}{}>",
                match self.table_state {
                    TableState::Head => "th",
                    TableState::Body => "td",
                },
                match self.table_alignments.get(self.table_cell_index) {
                    Some(Alignment::Left) => r#" align="left""#,
                    Some(Alignment::Right) => r#" align="right""#,
                    Some(Alignment::Center) => r#" align="center""#,
                    _ => "",
                }
            ),
        }
    }

    fn on_end<W: StrWrite>(&mut self, w: &mut W, tag: Tag) -> io::Result<()> {
        match tag {
            Tag::BlockQuote => w.write_str("</blockquote>"),
            Tag::CodeBlock(_) => w.write_str("</code></pre>"),
            Tag::Emphasis => w.write_str("</em>"),
            Tag::FootnoteDefinition(_) => w.write_str("</div>"),
            Tag::Heading(level) => write!(w, "</h{}>", level),
            Tag::Image(_, _, _) => Ok(()), /* shouldn't happen, handled in
                                             * start */
            Tag::Item => w.write_str("</li>"),
            Tag::Link(_, _, _) => w.write_str("</a>"),
            Tag::List(Some(_)) => w.write_str("</ol>"),
            Tag::List(None) => w.write_str("</ul>"),
            Tag::Paragraph => w.write_str("</p>"),
            Tag::Strikethrough => w.write_str("</del>"),
            Tag::Strong => w.write_str("</strong>"),
            Tag::Table(_) => w.write_str("</tbody></table>"),
            Tag::TableHead => {
                self.table_state = TableState::Body;
                w.write_str("</tr></thead><tbody>")
            }
            Tag::TableRow => w.write_str("</tr>"),
            Tag::TableCell => {
                self.table_cell_index += 1;
                w.write_str(match self.table_state {
                    TableState::Head => "</th>",
                    TableState::Body => "</td>",
                })
            }
        }
    }

    fn on_text<W: StrWrite>(
        &mut self,
        w: &mut W,
        s: CowStr,
    ) -> io::Result<()> {
        escape_html(w, &s)
    }

    fn on_code<W: StrWrite>(
        &mut self,
        w: &mut W,
        s: CowStr,
    ) -> io::Result<()> {
        write!(w, "<code>{}</code>", EscapeHtml(s))
    }

    fn on_html<W: StrWrite>(
        &mut self,
        w: &mut W,
        s: CowStr,
    ) -> io::Result<()> {
        w.write_str(&s)
    }

    fn on_soft_break<W: StrWrite>(&mut self, w: &mut W) -> io::Result<()> {
        w.write_str("\n")
    }

    fn on_hard_break<W: StrWrite>(&mut self, w: &mut W) -> io::Result<()> {
        w.write_str("<br />")
    }

    fn on_rule<W: StrWrite>(&mut self, w: &mut W) -> io::Result<()> {
        w.write_str("<hr />")
    }

    fn on_task_list_marker<W: StrWrite>(
        &mut self,
        w: &mut W,
        checked: bool,
    ) -> io::Result<()> {
        write!(
            w,
            r#"<input disabled="" type="checkbox" {}/>"#,
            match checked {
                true => r#"checked="" "#,
                false => "",
            }
        )
    }
}

/// Converts [`Event`]s into an HTML string much like
/// `pulldown_cmark::html::push_html` except that this also supports footnote
/// prefixes. See the module description for more details.
pub fn push_html<'a, I>(
    out: &mut String,
    events: I,
    footnote_prefix: &str,
) -> io::Result<()>
where
    I: Iterator<Item = Event<'a>>,
{
    let mut renderer = HtmlRenderer::with_footnote_prefix(footnote_prefix);
    for event in events {
        renderer.on_event(out, event)?;
    }
    Ok(())
}
