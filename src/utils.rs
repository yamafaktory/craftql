use crate::config::ALLOWED_EXTENSIONS;
use crate::state::{Data, Entity, GraphQLType, Node};

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

fn is_extension_allowed(extension: &str) -> bool {
    ALLOWED_EXTENSIONS.to_vec().contains(&extension)
}

/// Recursively read directories and files for a given path.
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

/// Parse the files, generate an AST and walk it to populate the graph.
pub async fn create_graph(shared_data: Arc<Mutex<Data>>) -> Result<()> {
    let mut data = shared_data.lock().await;
    let files = &data.files.clone();

    for (file, contents) in files {
        let ast = parse_schema::<String>(contents.as_str())?;

        for definition in ast.definitions {
            match definition {
                schema::Definition::TypeDefinition(type_definition) => match type_definition {
                    schema::TypeDefinition::Enum(todo) => {}
                    schema::TypeDefinition::InputObject(todo) => {}
                    schema::TypeDefinition::Interface(todo) => {}
                    schema::TypeDefinition::Object(object_type) => {
                        dbg!(object_type.to_string());
                        data.graph.add_node(Node {
                            id: file.to_owned(),
                            entity: Entity::new(
                                object_type
                                    .fields
                                    .iter()
                                    .map(|field| {
                                        walk_field(field)
                                        // dbg!(walk_field(field));
                                        // data.graph.add_node(Node {
                                        //     id: field.name.to_owned(),
                                        //     inner: (),
                                        // });
                                    })
                                    .collect::<Vec<String>>(),
                                GraphQLType::Object,
                                object_type.name,
                                contents.to_owned(),
                            ),
                        });
                        // object_type.fields
                        //     .iter()
                        //     .map(|field| {
                        //         dbg!(walk_field(field));
                        //         data.graph.add_node(Node {
                        //             id: field.name.to_owned(),
                        //             inner: (),
                        //         });
                        //     })
                        //     .collect::<()>();
                    }
                    schema::TypeDefinition::Scalar(todo) => {}
                    schema::TypeDefinition::Union(todo) => {}
                },
                schema::Definition::SchemaDefinition(todo) => {}
                schema::Definition::DirectiveDefinition(todo) => {}
                schema::Definition::TypeExtension(todo) => {}
            }
        }
    }

    Ok(())
}

/// Recursively walk the field types to get the inner String value.
fn walk_field(field: &schema::Field<String>) -> String {
    fn walk_field_type(field_type: &schema::Type<String>) -> String {
        match field_type {
            schema::Type::NamedType(name) => name.clone(),
            schema::Type::ListType(field_type) => {
                // Field type is boxed, need to unbox.
                walk_field_type(field_type.as_ref())
            }
            schema::Type::NonNullType(field_type) => {
                // Same here.
                walk_field_type(field_type.as_ref())
            }
        }
    };

    walk_field_type(&field.field_type)
}
