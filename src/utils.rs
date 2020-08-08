use crate::config::ALLOWED_EXTENSIONS;
use crate::state::Data;

use anyhow::Result;
use async_std::{
    fs,
    path::Path,
    prelude::*,
    sync::{Arc, Mutex},
};
use std::{future::Future, pin::Pin};

fn is_extension_allowed(extension: &str) -> bool {
    ALLOWED_EXTENSIONS.to_vec().contains(&extension)
}

pub fn get_files(
    path: String,
    shared_data: Arc<Mutex<Data>>,
) -> Pin<Box<dyn Future<Output = Result<()>>>> {
    // Use a hack to get async recursive calls working.
    Box::pin(async move {
        let thread_safe_path = Arc::new(path);
        let file_or_dir = fs::metadata(thread_safe_path.as_ref()).await?;
        let file_type = file_or_dir.file_type();

        if file_type.is_file() {
            if is_extension_allowed(
                Path::new(thread_safe_path.as_ref())
                    .extension()
                    .unwrap()
                    .to_str()
                    .unwrap(),
            ) {
                let contents = fs::read_to_string(thread_safe_path.as_ref()).await?;
                let mut data = shared_data.lock().await;

                data.files
                    .insert(thread_safe_path.as_ref().clone(), contents);
            }

            return Ok(());
        }

        let mut dir = fs::read_dir(thread_safe_path.as_ref()).await?;

        while let Some(res) = dir.next().await {
            let entry: fs::DirEntry = res?;
            let inner_path = entry.path();
            let inner_path_cloned = inner_path.clone();
            let metadata = entry.clone().metadata().await?;
            let is_dir = metadata.is_dir();
            let inner_path_as_string = inner_path_cloned.into_os_string().into_string().unwrap();

            if !is_dir && is_extension_allowed(&inner_path.extension().unwrap().to_str().unwrap()) {
                let contents = fs::read_to_string(inner_path).await?;
                let mut data = shared_data.lock().await;

                data.files.insert(inner_path_as_string, contents);
            } else {
                get_files(inner_path_as_string, shared_data.clone()).await?;
            }
        }

        Ok(())
    })
}

// TODO:
// use graphql_parser::parse_schema;
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
