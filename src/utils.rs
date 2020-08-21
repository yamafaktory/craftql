use crate::config::ALLOWED_EXTENSIONS;
use crate::state::{Data, Entity, GraphQL, GraphQLType, Node};

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

fn get_extended_id(id: String) -> String {
    format!("{}Ext", id)
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
                    schema::TypeDefinition::Enum(enum_type) => {
                        let id = enum_type.name.clone();
                        let dependencies = enum_type
                            .directives
                            .iter()
                            .map(|directive| directive.name.clone())
                            .chain(
                                enum_type
                                    .values
                                    .iter()
                                    .map(|enum_value| {
                                        enum_value
                                            .directives
                                            .iter()
                                            .map(|directive| directive.name.clone())
                                    })
                                    .flatten(),
                            )
                            .collect::<Vec<String>>();

                        let node_index = data.graph.add_node(Node::new(
                            Entity::new(
                                dependencies.clone(), // Enums don't have dependencies.
                                GraphQL::TypeDefinition(GraphQLType::Enum),
                                enum_type.name,
                                file.to_owned(),
                                contents.to_owned(),
                            ),
                            id,
                        ));

                        // Update dependencies.
                        dependency_hash_map.insert(node_index, dependencies);
                    }
                    schema::TypeDefinition::InputObject(input_object_type) => {
                        let id = input_object_type.name.clone();
                        let fields = input_object_type
                            .fields
                            .iter()
                            .map(|input_value| walk_input_value(input_value))
                            .flatten()
                            .chain(
                                input_object_type
                                    .directives
                                    .iter()
                                    .map(|directive| directive.name.clone()),
                            )
                            .collect::<Vec<String>>();

                        // Inject entity as node into the graph.
                        let node_index = data.graph.add_node(Node::new(
                            Entity::new(
                                fields.clone(),
                                GraphQL::TypeDefinition(GraphQLType::InputObject),
                                input_object_type.name,
                                file.to_owned(),
                                contents.to_owned(),
                            ),
                            id,
                        ));

                        // Update dependencies.
                        dependency_hash_map.insert(node_index, fields);
                    }
                    schema::TypeDefinition::Interface(interface_type) => {
                        let id = interface_type.name.clone();
                        let fields = interface_type
                            .fields
                            .iter()
                            .map(|field| walk_field(field))
                            .into_iter()
                            .flatten()
                            .collect::<Vec<String>>();

                        // Inject entity as node into the graph.
                        let node_index = data.graph.add_node(Node::new(
                            Entity::new(
                                fields.clone(),
                                GraphQL::TypeDefinition(GraphQLType::Interface),
                                interface_type.name,
                                file.to_owned(),
                                contents.to_owned(),
                            ),
                            id,
                        ));

                        // Update dependencies.
                        dependency_hash_map.insert(node_index, fields);
                    }
                    schema::TypeDefinition::Object(object_type) => {
                        let id = object_type.name.clone();
                        // Get both the fields and the interfaces from the object.
                        let fields_and_interfaces = vec![
                            object_type
                                .fields
                                .iter()
                                .map(|field| walk_field(field))
                                .into_iter()
                                .flatten()
                                .collect::<Vec<String>>(),
                            object_type.implements_interfaces,
                        ]
                        .into_iter()
                        .flatten()
                        .collect::<Vec<String>>();

                        // Inject entity as node into the graph.
                        let node_index = data.graph.add_node(Node::new(
                            Entity::new(
                                fields_and_interfaces.clone(),
                                GraphQL::TypeDefinition(GraphQLType::Object),
                                object_type.name,
                                file.to_owned(),
                                contents.to_owned(),
                            ),
                            id,
                        ));

                        // Update dependencies.
                        dependency_hash_map.insert(node_index, fields_and_interfaces);
                    }
                    schema::TypeDefinition::Scalar(scalar_type) => {
                        let id = scalar_type.name.clone();

                        data.graph.add_node(Node::new(
                            Entity::new(
                                vec![],
                                GraphQL::TypeDefinition(GraphQLType::Scalar),
                                scalar_type.name,
                                file.to_owned(),
                                contents.to_owned(),
                            ),
                            id,
                        ));
                    }
                    schema::TypeDefinition::Union(union_type) => {
                        let id = union_type.name.clone();
                        let types = union_type.types;

                        // Inject entity as node into the graph.
                        let node_index = data.graph.add_node(Node::new(
                            Entity::new(
                                types.clone(),
                                GraphQL::TypeDefinition(GraphQLType::Union),
                                union_type.name,
                                file.to_owned(),
                                contents.to_owned(),
                            ),
                            id,
                        ));

                        // Update dependencies.
                        dependency_hash_map.insert(node_index, types);
                    }
                },
                schema::Definition::SchemaDefinition(schema_definition) => {
                    // A Schema has no name, use a default one.
                    let id = String::from("Schema");
                    // A schema can only have a query, a mutation and a subscription.
                    let fields = vec![
                        schema_definition.query,
                        schema_definition.mutation,
                        schema_definition.subscription,
                    ]
                    .into_iter()
                    .filter_map(|field| field)
                    .collect::<Vec<String>>();

                    // Inject entity as node into the graph.
                    let node_index = data.graph.add_node(Node::new(
                        Entity::new(
                            fields.clone(),
                            GraphQL::Schema,
                            String::from("Schema"),
                            file.to_owned(),
                            contents.to_owned(),
                        ),
                        id,
                    ));

                    // Update dependencies.
                    dependency_hash_map.insert(node_index, fields);
                }
                schema::Definition::DirectiveDefinition(directive_definition) => {
                    let id = directive_definition.name.clone();
                    let fields = directive_definition
                        .arguments
                        .iter()
                        .map(|input_value| walk_input_value(input_value))
                        .flatten()
                        .collect::<Vec<String>>();

                    // Inject entity as node into the graph.
                    let node_index = data.graph.add_node(Node::new(
                        Entity::new(
                            fields.clone(),
                            GraphQL::Directive,
                            directive_definition.name,
                            file.to_owned(),
                            contents.to_owned(),
                        ),
                        id,
                    ));

                    // Update dependencies.
                    dependency_hash_map.insert(node_index, fields);
                }
                schema::Definition::TypeExtension(type_extension) => {
                    // http://spec.graphql.org/draft/#SchemaExtension
                    match type_extension {
                        schema::TypeExtension::Object(object_type_extension) => {
                            let id = object_type_extension.name.clone();
                            // Merge the fields, the interfaces and the extended source.
                            let dependencies = vec![
                                object_type_extension
                                    .fields
                                    .iter()
                                    .map(|field| walk_field(field))
                                    .into_iter()
                                    .flatten()
                                    .collect::<Vec<String>>(),
                                object_type_extension.implements_interfaces,
                            ]
                            .into_iter()
                            .flatten()
                            .chain(vec![object_type_extension.name.clone()])
                            .collect::<Vec<String>>();

                            // Inject entity as node into the graph.
                            let node_index = data.graph.add_node(Node::new(
                                Entity::new(
                                    dependencies.clone(),
                                    GraphQL::TypeExtension(GraphQLType::Object),
                                    object_type_extension.name,
                                    file.to_owned(),
                                    contents.to_owned(),
                                ),
                                get_extended_id(id),
                            ));

                            // Update dependencies.
                            dependency_hash_map.insert(node_index, dependencies);
                        }
                        schema::TypeExtension::Scalar(scalar_type_extension) => {
                            let id = scalar_type_extension.name.clone();

                            let dependencies = scalar_type_extension
                                .directives
                                .iter()
                                .map(|directive| directive.name.clone())
                                .chain(vec![scalar_type_extension.name.clone()])
                                .collect::<Vec<String>>();

                            let node_index = data.graph.add_node(Node::new(
                                Entity::new(
                                    dependencies.clone(),
                                    GraphQL::TypeExtension(GraphQLType::Scalar),
                                    scalar_type_extension.name,
                                    file.to_owned(),
                                    contents.to_owned(),
                                ),
                                get_extended_id(id),
                            ));

                            // Update dependencies.
                            dependency_hash_map.insert(node_index, dependencies);
                        }
                        schema::TypeExtension::Interface(interface_type_extension) => {
                            let id = interface_type_extension.name.clone();

                            let dependencies = interface_type_extension
                                .directives
                                .iter()
                                .map(|directive| directive.name.clone())
                                // Inject fields.
                                .chain(
                                    interface_type_extension
                                        .fields
                                        .iter()
                                        .map(|field| walk_field(field))
                                        .into_iter()
                                        .flatten(),
                                )
                                // Inject source.
                                .chain(vec![interface_type_extension.name.clone()])
                                .collect::<Vec<String>>();

                            let node_index = data.graph.add_node(Node::new(
                                Entity::new(
                                    dependencies.clone(),
                                    GraphQL::TypeExtension(GraphQLType::Scalar),
                                    interface_type_extension.name,
                                    file.to_owned(),
                                    contents.to_owned(),
                                ),
                                get_extended_id(id),
                            ));

                            // Update dependencies.
                            dependency_hash_map.insert(node_index, dependencies);
                        }
                        schema::TypeExtension::Union(union_type_extension) => {
                            let id = union_type_extension.name.clone();
                            let mut dependencies = union_type_extension.types;

                            dependencies.extend(vec![union_type_extension.name.clone()]);

                            // Inject entity as node into the graph.
                            let node_index = data.graph.add_node(Node::new(
                                Entity::new(
                                    dependencies.clone(),
                                    GraphQL::TypeExtension(GraphQLType::Union),
                                    union_type_extension.name,
                                    file.to_owned(),
                                    contents.to_owned(),
                                ),
                                get_extended_id(id),
                            ));

                            // Update dependencies.
                            dependency_hash_map.insert(node_index, dependencies);
                        }
                        schema::TypeExtension::Enum(enum_type_extension) => {
                            let id = enum_type_extension.name.clone();

                            // Enums don't have dependencies but here we need the enum source.
                            let dependencies = vec![enum_type_extension.name.clone()];

                            let node_index = data.graph.add_node(Node::new(
                                Entity::new(
                                    dependencies.clone(),
                                    GraphQL::TypeExtension(GraphQLType::Enum),
                                    enum_type_extension.name,
                                    file.to_owned(),
                                    contents.to_owned(),
                                ),
                                get_extended_id(id),
                            ));

                            // Update dependencies.
                            dependency_hash_map.insert(node_index, dependencies);
                        }
                        schema::TypeExtension::InputObject(input_object_type_extension) => {
                            let id = input_object_type_extension.name.clone();
                            let dependencies = input_object_type_extension
                                .fields
                                .iter()
                                .map(|input_value| walk_input_value(input_value))
                                .flatten()
                                .chain(vec![input_object_type_extension.name.clone()])
                                .collect::<Vec<String>>();

                            // Inject entity as node into the graph.
                            let node_index = data.graph.add_node(Node::new(
                                Entity::new(
                                    dependencies.clone(),
                                    GraphQL::TypeExtension(GraphQLType::InputObject),
                                    input_object_type_extension.name,
                                    file.to_owned(),
                                    contents.to_owned(),
                                ),
                                get_extended_id(id),
                            ));

                            // Update dependencies.
                            dependency_hash_map.insert(node_index, dependencies);
                        }
                    };
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

/// Recursively walk a field type to get the inner String value.
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

/// Recursively walk a field to get the inner String value and the arguments.
fn walk_field(field: &schema::Field<String>) -> Vec<String> {
    field
        // Inject arguments.
        .arguments
        .iter()
        .map(|argument| walk_field_type(&argument.value_type))
        .into_iter()
        // Inject directives.
        .chain(
            field
                .directives
                .iter()
                .map(|directive| directive.name.clone()),
        )
        // Inject field type.
        .chain(vec![walk_field_type(&field.field_type)])
        .collect::<Vec<String>>()
}

/// Recursively walk an input to get the inner String value.
fn walk_input_value(input_value: &schema::InputValue<String>) -> Vec<String> {
    input_value
        .directives
        .iter()
        .map(|directive| directive.name.clone())
        .into_iter()
        .chain(vec![walk_field_type(&input_value.value_type)])
        .collect::<Vec<String>>()
}
