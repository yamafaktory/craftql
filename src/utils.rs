use crate::config::ALLOWED_EXTENSIONS;
use crate::state::{Data, Entity, GraphQLDefinition, GraphQLType, Node};

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
use petgraph::graph::NodeIndex;
use std::collections::HashMap;

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
    // Keep track of the dependencies for edges.
    let mut dependency_hash_map: HashMap<NodeIndex, Vec<String>> = HashMap::new();

    // Populate the nodes first.
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
                        let id = input_object.name.clone();
                        let fields = input_object
                            .fields
                            .iter()
                            .map(|input_value| walk_input_value(input_value))
                            .collect::<Vec<String>>();

                        // Inject entity as node into the graph.
                        let node_index = data.graph.add_node(Node::new(
                            Entity::new(
                                fields.clone(),
                                GraphQLType::InputObject,
                                input_object.name,
                                file.to_owned(),
                                contents.to_owned(),
                            ),
                            id,
                        ));

                        // Update dependencies.
                        dependency_hash_map.insert(node_index, fields);
                    }
                    schema::TypeDefinition::Interface(interface) => {
                        let id = interface.name.clone();
                        let fields = interface
                            .fields
                            .iter()
                            .map(|field| walk_field(field))
                            .collect::<Vec<String>>();

                        // Inject entity as node into the graph.
                        let node_index = data.graph.add_node(Node::new(
                            Entity::new(
                                fields.clone(),
                                GraphQLType::Interface,
                                interface.name,
                                file.to_owned(),
                                contents.to_owned(),
                            ),
                            id,
                        ));

                        // Update dependencies.
                        dependency_hash_map.insert(node_index, fields);
                    }
                    schema::TypeDefinition::Object(object) => {
                        let id = object.name.clone();
                        // Get both the fields and the interfaces from the object.
                        let fields_and_interfaces = vec![
                            object
                                .fields
                                .iter()
                                .map(|field| walk_field(field))
                                .collect::<Vec<String>>(),
                            object.implements_interfaces,
                        ]
                        .into_iter()
                        .flatten()
                        .collect::<Vec<String>>();

                        // Inject entity as node into the graph.
                        let node_index = data.graph.add_node(Node::new(
                            Entity::new(
                                fields_and_interfaces.clone(),
                                GraphQLType::Object,
                                object.name,
                                file.to_owned(),
                                contents.to_owned(),
                            ),
                            id,
                        ));

                        // Update dependencies.
                        dependency_hash_map.insert(node_index, fields_and_interfaces);
                    }
                    schema::TypeDefinition::Scalar(scalar) => {
                        let id = scalar.name.clone();

                        data.graph.add_node(Node::new(
                            Entity::new(
                                vec![],
                                GraphQLType::Object,
                                scalar.name,
                                file.to_owned(),
                                contents.to_owned(),
                            ),
                            id,
                        ));
                    }
                    schema::TypeDefinition::Union(union) => {
                        let id = union.name.clone();
                        let types = union.types;

                        // Inject entity as node into the graph.
                        let node_index = data.graph.add_node(Node::new(
                            Entity::new(
                                types.clone(),
                                GraphQLType::Union,
                                union.name,
                                file.to_owned(),
                                contents.to_owned(),
                            ),
                            id,
                        ));

                        // Update dependencies.
                        dependency_hash_map.insert(node_index, types);
                    }
                },
                schema::Definition::SchemaDefinition(schema) => {
                    // A Schema has no name, use a default one.
                    let id = String::from("Schema");
                    // A schema can only have a query, a mutation and a subscription.
                    let fields = vec![schema.query, schema.mutation, schema.subscription]
                        .into_iter()
                        .filter_map(|field| field)
                        .collect::<Vec<String>>();

                    // Inject entity as node into the graph.
                    let node_index = data.graph.add_node(Node::new(
                        Entity::new(
                            fields.clone(),
                            GraphQLType::Definition(GraphQLDefinition::Schema),
                            String::from("Schema"),
                            file.to_owned(),
                            contents.to_owned(),
                        ),
                        id,
                    ));

                    // Update dependencies.
                    dependency_hash_map.insert(node_index, fields);
                }
                schema::Definition::DirectiveDefinition(directive) => {
                    // dbg!(todo);
                }
                schema::Definition::TypeExtension(type_extension) => {
                    // dbg!(todo);
                }
            }
        }
    }

    // Populate the edges.
    for (node_index, dependencies) in dependency_hash_map {
        for dependency in dependencies {
            // https://docs.rs/petgraph/0.5.1/petgraph/graph/struct.Graph.html#method.node_indices
            let maybe_index = &data
                .graph
                .node_indices()
                .find(|index| data.graph[*index].id == dependency);

            if let Some(index) = *maybe_index {
                &data
                    .graph
                    .update_edge(index, node_index, (index, node_index));
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
