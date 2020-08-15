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
pub async fn populate_graph_from_ast(shared_data: Arc<Mutex<Data>>) -> Result<()> {
    let mut data = shared_data.lock().await;
    let files = &data.files.clone();

    for (file, contents) in files {
        let ast = parse_schema::<String>(contents.as_str())?;

        for definition in ast.definitions {
            match definition {
                schema::Definition::TypeDefinition(type_definition) => match type_definition {
                    schema::TypeDefinition::Enum(inner_enum) => {
                        let id = inner_enum.name.clone();

                        data.graph.add_node(Node::new(
                            Entity::new(
                                vec![],
                                GraphQLType::Enum,
                                inner_enum.name,
                                file.to_owned(),
                                contents.to_owned(),
                            ),
                            id,
                        ));
                    }
                    schema::TypeDefinition::InputObject(input_object) => {
                        let fields = input_object
                            .fields
                            .iter()
                            .map(|input_value| {
                                let input_value = walk_input_value(input_value);

                                // TODO: keep track of edge entity -> field.

                                input_value.clone()
                            })
                            .collect::<Vec<String>>();

                        let id = input_object.name.clone();

                        // Inject entity as node into the graph.
                        data.graph.add_node(Node::new(
                            Entity::new(
                                fields,
                                GraphQLType::InputObject,
                                input_object.name,
                                file.to_owned(),
                                contents.to_owned(),
                            ),
                            id,
                        ));
                    }
                    schema::TypeDefinition::Interface(todo) => {}
                    schema::TypeDefinition::Object(object_type) => {
                        // ----------------------------------
                        // TODO: take care of `implements`!
                        // Those will need to be tracked as edges.
                        // ----------------------------------
                        dbg!(object_type.implements_interfaces);
                        let fields = object_type
                            .fields
                            .iter()
                            .map(|field| {
                                let field = walk_field(field);

                                // TODO: keep track of edge entity -> field.

                                field.clone()
                            })
                            .collect::<Vec<String>>();
                        let id = object_type.name.clone();

                        // Inject entity as node into the graph.
                        data.graph.add_node(Node::new(
                            Entity::new(
                                fields,
                                GraphQLType::Object,
                                object_type.name,
                                file.to_owned(),
                                contents.to_owned(),
                            ),
                            id,
                        ));
                    }
                    schema::TypeDefinition::Scalar(todo) => {}
                    schema::TypeDefinition::Union(todo) => {}
                },
                schema::Definition::SchemaDefinition(todo) => {
                    dbg!(todo);
                }
                schema::Definition::DirectiveDefinition(todo) => {
                    dbg!(todo);
                }
                schema::Definition::TypeExtension(todo) => {
                    dbg!(todo);
                }
            }
        }
    }

    Ok(())
}

/// Recursively walk the field types to get the inner String value.
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
}

/// Recursively walk a field to get the inner String value.
fn walk_field(field: &schema::Field<String>) -> String {
    walk_field_type(&field.field_type)
}

/// Recursively walk an input to get the inner String value.
fn walk_input_value(input_value: &schema::InputValue<String>) -> String {
    walk_field_type(&input_value.value_type)
}
