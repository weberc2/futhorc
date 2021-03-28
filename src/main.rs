#![feature(array_windows)]

use crate::build::*;
use crate::url::Url;
use std::path::Path;

mod build;
mod page;
mod post;
mod slice;
mod url;
mod value;
mod write;

fn main() {
    build_site(&Config {
        source_directory: Path::new(match &*std::env::args().collect::<Vec<String>>() {
            [_, path, ..] => path.as_str(),
            _ => "./test-data/posts/",
        }),
        site_root: Url::new("file:///tmp/pages/0.html"),
        index_url: Url::new("file:///tmp/pages"),
        index_template: INDEX_TEMPLATE,
        index_directory: Path::new("/tmp/pages"),
        index_page_size: 2,
        posts_url: Url::new("file:///tmp/posts"),
        posts_template: POST_TEMPLATE,
        posts_directory: Path::new("/tmp/posts"),
        threads: Some(1),
    })
    .unwrap();
}

const POST_TEMPLATE: &str = r#"<html>
<body>
    <h1><a href={{.site_root}}>Craig Weber</a></h1>
    {{- with .page }}
    <div>
        <h2>{{.item.title}}</h2>
        <p>{{.item.date}}</p>
        {{range .item.tags}}<a href={{.url}}><{{.tag}}</url>{{end}}
        {{.item.body}}
    </div>
    {{- if .prev }}
    <a href={{.prev}}>Previous</a>
    {{- end }}
    {{- if .next }}
    <a href={{.next}}>Next</a>
    {{- end }}
    {{- end }}
</body>
</html>"#;

const INDEX_TEMPLATE: &str = r#"<html>
<body>
    <h1><a href={{.site_root}}>Craig Weber</a></h1>
    {{- with .page }}
    {{- range .item }}
        <div>
            <h2><a href={{.url}}>{{.title}}</a></h2>
            <p>{{.date}}</p>
            {{range .tags}}<a href={{.url}}>{{.tag}}</a>{{end}}
            {{.summary}}
            {{- if .summarized }}
            <a href={{.url}}>Read More</a>
            {{- end }}
        </div>
    {{- end }}
    {{- if .prev }}
    <a href={{.prev}}>Prev Page</a>
    {{- end}}
    {{- if .next }}
    <a href={{.next}}>Next Page</a>
    {{- end}}
    {{- end}}
</body>
</html>"#;
