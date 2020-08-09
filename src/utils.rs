use crate::config::ALLOWED_EXTENSIONS;
use crate::state::{Data, Node};

use anyhow::Result;
use async_std::{
    fs,
    future::Future,
    path::Path,
    pin::Pin,
    prelude::*,
    sync::{Arc, Mutex},
};
use graphql_parser::{parse_schema, schema};
use petgraph::dot::Dot;

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

pub async fn foo(shared_data: Arc<Mutex<Data>>) -> Result<()> {
    let mut data = shared_data.lock().await;

    for (file, contents) in &data.files {
        let ast = parse_schema::<String>(contents.as_str())?;

        for definition in ast.definitions {
            match definition {
                schema::Definition::TypeDefinition(t) => match t {
                    schema::TypeDefinition::Scalar(t) => {}
                    schema::TypeDefinition::Object(t) => {
                        // dbg!(&t);
                        // &data.graph.add_node(Node {
                        //     id: String::from("TODO"),
                        //     todo: (),
                        // });
                        t.fields
                            .iter()
                            .map(|field| {
                                dbg!(&field);
                            })
                            .collect::<()>();
                    }
                    schema::TypeDefinition::Interface(t) => {}
                    schema::TypeDefinition::Union(t) => {}
                    schema::TypeDefinition::Enum(t) => {}
                    schema::TypeDefinition::InputObject(t) => {}
                },
                schema::Definition::SchemaDefinition(t) => {}
                schema::Definition::DirectiveDefinition(t) => {}
                schema::Definition::TypeExtension(t) => {}
            }
        }
    }

    println!("{:?}", Dot::new(&data.graph));

    Ok(())
}
