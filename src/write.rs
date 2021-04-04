use anyhow::{anyhow, Result};
use gtmpl::Template;
use gtmpl_value::Value;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

use crate::page::*;
use crate::url::Url;

struct Context<'a, T2> {
    page: Page<T2>,
    home_page: &'a Url,
    static_url: &'a Url,
}

impl<'a, T2> From<Context<'a, T2>> for Value
where
    T2: Into<Value>,
{
    fn from(c: Context<'a, T2>) -> Value {
        let mut m: HashMap<String, Value> = HashMap::new();
        m.insert("page".to_owned(), c.page.into());
        m.insert("home_page".to_owned(), c.home_page.into());
        m.insert("static_url".to_owned(), c.static_url.into());
        Value::Object(m)
    }
}

fn write_pages_singlethreaded<T, I>(
    pages: I,
    directory: &Path,
    template: &Template,
    home_page: &Url,
    static_url: &Url,
) -> Result<()>
where
    Value: From<T>,
    I: Iterator<Item = Page<T>>,
{
    fs::create_dir_all(directory)?;
    for context in pages.map(|page| Context {
        page,
        home_page,
        static_url,
    }) {
        let path = directory.join(format!("{}.html", context.page.id));
        let ctx = match gtmpl::Context::from(context) {
            Ok(ctx) => Ok(ctx),
            Err(e) => Err(anyhow!(e)),
        }?;
        let mut file = fs::File::create(path)?;
        if let Err(e) = template.execute(&mut file, &ctx) {
            return Err(anyhow!(e));
        }
    }
    Ok(())
}

fn write_pages_parallel<T, I>(
    pages: I,
    directory: &Path,
    template: &Template,
    home_page: &Url,
    static_url: &Url,
    threads: usize,
) -> Result<()>
where
    Value: From<T>,
    T: Sync + Send,
    I: Iterator<Item = Page<T>>,
{
    fs::create_dir_all(directory)?;

    use crossbeam_channel::unbounded;

    let (tx, rx) = unbounded::<Page<T>>();

    if let Err(e) = crossbeam::scope(|scope| -> Result<()> {
        let mut handles = Vec::with_capacity(threads);
        for _ in 0..handles.capacity() {
            let rx = rx.clone();
            handles.push(scope.spawn(move |_| -> Result<()> {
                for page in rx {
                    let path = directory.join(format!("{}.html", page.id));
                    let context = match gtmpl::Context::from(Context {
                        page,
                        home_page,
                        static_url,
                    }) {
                        Ok(ctx) => Ok(ctx),
                        Err(e) => Err(anyhow!(e)),
                    }?;
                    let mut file = fs::File::create(path)?;
                    if let Err(e) = template.execute(&mut file, &context) {
                        return Err(anyhow!(e));
                    }
                }
                Ok(())
            }));
        }

        for page in pages {
            tx.send(page).unwrap();
        }
        drop(tx);

        for handle in handles {
            handle.join().unwrap()?;
        }

        Ok(())
    }) {
        std::panic::resume_unwind(e);
    }
    Ok(())
}

pub fn write_pages<T, I>(
    pages: I,
    directory: &Path,
    template: &Template,
    home_page: &Url,
    static_url: &Url,
    threads: usize,
) -> Result<()>
where
    Value: From<T>,
    T: Sync + Send,
    I: Iterator<Item = Page<T>>,
{
    if threads < 2 {
        write_pages_singlethreaded(pages, directory, template, home_page, static_url)
    } else {
        write_pages_parallel(pages, directory, template, home_page, static_url, threads)
    }
}
