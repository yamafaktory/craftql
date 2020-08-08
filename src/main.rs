mod config;

use crate::config::ALLOWED_EXTENSIONS;

use anyhow::Result;
use async_std::fs;
use async_std::prelude::*;
use clap::{crate_authors, crate_description, crate_version, Clap};
use graphql_parser::parse_schema;
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

#[derive(Clap)]
#[clap(author = crate_authors!(), about = crate_description!(), version = crate_version!())]
struct Opts {
    path: String,
}

fn foo(path: String) -> Pin<Box<dyn Future<Output = Result<()>>>> {
    // Use a hack to get async recursive calls working.
    Box::pin(async move {
        let thread_safe_path = Arc::new(path);
        let mut files: HashMap<String, String> = HashMap::new();
        let file_or_dir = fs::metadata(thread_safe_path.as_ref()).await?;
        let file_type = file_or_dir.file_type();

        if file_type.is_file() {
            let contents = fs::read_to_string(thread_safe_path.as_ref()).await?;

            files.insert(String::from("TODO"), contents);

            return Ok(());
        }

        let mut dir = fs::read_dir(thread_safe_path.as_ref()).await?;

        while let Some(res) = dir.next().await {
            let entry: fs::DirEntry = res?;
            let inner_path = entry.path();
            let file_name = entry.file_name();
            let name = file_name.to_string_lossy();
            let metadata = entry.clone().metadata().await?;
            let is_dir = metadata.is_dir();

            if !is_dir
                && ALLOWED_EXTENSIONS
                    .to_vec()
                    .contains(&inner_path.extension().unwrap().to_str().unwrap())
            {
                let contents = fs::read_to_string(inner_path).await?;

                files.insert(String::from(name), contents);
            } else {
                foo(inner_path.into_os_string().into_string().unwrap()).await?;
            }
        }

        dbg!(files);

        return Ok(());
    })
}

#[async_std::main]
async fn main() -> Result<()> {
    let opts: Opts = Opts::parse();
    let path = opts.path;

    foo(path).await

    //     let ast = parse_schema::<String>(
    //         r#"
    //     type User {
    //         "The user's URN"
    //         urn: URN!
    //         foo: BAR
    //     }
    // "#,
    //     );
    //     ast.map(|node| {
    //         if let t = Some(node) {
    //             match t {
    //                 Some(t) => {
    //                     dbg!(t.definitions);
    //                 }
    //                 None => {}
    //             };
    //         }
    //     });
}
