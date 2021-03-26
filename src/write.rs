use std::collections::HashMap;
use std::fs;
use std::path::Path;
use anyhow::{anyhow, Result};
use gtmpl_value::Value;

use crate::page::*;

pub fn write_pages<T, I>(
    pages: I,
    directory: &str,
    template: &str,
    site_root: &str,
) -> Result<()>
where
    Value: From<T>,
    T: Sync + Send,
    I: Iterator<Item = Page<T>>
{
        struct Context<'a, T2> {
            page: Page<T2>,
            site_root: &'a str,
        }

        impl<'a, T2> From<Context<'a, T2>> for Value where T2: Into<Value> {
            fn from(c: Context<'a, T2>) -> Value {
                let mut m: HashMap<String, Value> = HashMap::new();
                m.insert("page".to_owned(), c.page.into());
                m.insert("site_root".to_owned(), c.site_root.into());
                Value::Object(m)
            }
        }

        let directory = Path::new(directory);
        fs::create_dir_all(directory)?;

        use crossbeam_channel::unbounded;

        let (tx, rx) = unbounded::<Page<T>>();

        crossbeam::scope(|scope| -> Result<()> {
            let mut handles = Vec::with_capacity(8);
            for _ in 0..handles.capacity() {
                let rx = rx.clone();
                handles.push(scope.spawn(move |_| -> Result<()> {
                    let mut t = gtmpl::Template::default();
                    if let Err(e) = t.parse(template) {
                        return Err(anyhow!(e));
                    }
                    for page in rx {
                        let path = directory.join(format!("{}.html", page.id));
                        let context = match gtmpl::Context::from(Context{page: page, site_root: site_root}) {
                            Ok(ctx) => Ok(ctx),
                            Err(e) => Err(anyhow!(e)),
                        }?;
                        let mut file = fs::File::create(path)?;
                        if let Err(e) = t.execute(&mut file, &context) {
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
        }).unwrap()


        // let mut t = gtmpl::Template::default();
        // if let Err(e) = t.parse(template) {
        //     return Err(anyhow!(e));
        // }
        // for context in pages.map(|page| Context{page, site_root}) {
        //     let path = directory.join(format!("{}.html", context.page.id));
        //     let ctx = match gtmpl::Context::from(context) {
        //         Ok(ctx) => Ok(ctx),
        //         Err(e) => Err(anyhow!(e)),
        //     }?;
        //     let mut file = fs::File::create(path)?;
        //     if let Err(e) = t.execute(&mut file, &ctx) {
        //         return Err(anyhow!(e));
        //     }
        // }
        // Ok(())
}