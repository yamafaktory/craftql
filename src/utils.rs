use crate::config::ALLOWED_EXTENSIONS;
use crate::extend_types::ExtendType;
use crate::state::{Entity, GraphQL, GraphQLType, Node};

use anyhow::Result;
use async_std::{
    fs,
    future::Future,
    path::{Path, PathBuf},
    pin::Pin,
    prelude::*,
    sync::{Arc, Mutex},
};
use graphql_parser::{parse_schema, schema};
use petgraph::{graph::NodeIndex, Direction::Outgoing};
use std::{collections::HashMap, process::exit};

/// Check if a file extension is allowed.
fn is_extension_allowed(extension: &str) -> bool {
    ALLOWED_EXTENSIONS.to_vec().contains(&extension)
}

/// Find orphans node and display them.
pub async fn find_orphans(
    graph: Arc<Mutex<petgraph::Graph<Node, (NodeIndex, NodeIndex)>>>,
) -> Result<()> {
    let graph = graph.lock().await;
    let externals = graph.externals(Outgoing);
    let has_root_schema = graph
        .node_indices()
        .find(|index| graph[*index].id == "schema")
        .is_some();

    for index in externals {
        let entity = &graph.node_weight(index).unwrap().entity;

        match entity.graphql {
            // Skip root schema has it can't have outgoing edges.
            GraphQL::Schema => {}
            // Skip Mutation, Query and Subscription if no root schema is defined
            // as those nodes can't have outgoing edges.
            GraphQL::TypeDefinition(GraphQLType::Object)
                if (!has_root_schema
                    && (entity.name == String::from("Mutation")
                        || entity.name == String::from("Query")
                        || entity.name == String::from("Subscription"))) => {}
            _ => {
                println!("{}", entity);
            }
        };
    }

    Ok(())
}

/// Find a node by name, display it with syntax highlighting or exit.
pub async fn find_node(
    node: String,
    graph: Arc<Mutex<petgraph::Graph<Node, (NodeIndex, NodeIndex)>>>,
) -> Result<()> {
    let graph = graph.lock().await;

    match graph.node_indices().find(|index| graph[*index].id == *node) {
        Some(index) => {
            let entity = &graph.node_weight(index).unwrap().entity;

            println!("{}", entity);

            Ok(())
        }
        None => {
            eprintln!("Node {} not found", node);
            exit(1);
        }
    }
}

/// Recursively read directories and files for a given path.
pub fn get_files(
    path: PathBuf,
    files: Arc<Mutex<HashMap<PathBuf, String>>>,
) -> Pin<Box<dyn Future<Output = Result<()>>>> {
    // Use a hack to get async recursive calls working.
    Box::pin(async move {
        let thread_safe_path = Arc::new(path);
        let file_or_dir = fs::metadata(thread_safe_path.as_ref()).await?;
        let file_type = file_or_dir.file_type();
        let extension = match Path::new(thread_safe_path.as_ref()).extension() {
            Some(extension) => extension.to_str().unwrap(),
            None => "",
        };

        if file_type.is_file() {
            if is_extension_allowed(extension) {
                let contents = fs::read_to_string(thread_safe_path.as_ref()).await?;
                let mut files = files.lock().await;

                files.insert(thread_safe_path.as_ref().clone(), contents);
            }

            return Ok(());
        }

        let mut dir = fs::read_dir(thread_safe_path.as_ref()).await?;

        while let Some(result) = dir.next().await {
            let entry: fs::DirEntry = result?;
            let inner_path = entry.path();
            let inner_path_cloned = inner_path.clone();
            let metadata = entry.clone().metadata().await?;
            let is_dir = metadata.is_dir();
            let extension = match &inner_path.extension() {
                Some(extension) => extension.to_str().unwrap(),
                None => "",
            };

            if !is_dir && is_extension_allowed(extension) {
                let contents = fs::read_to_string(inner_path).await?;
                let mut files = files.lock().await;

                files.insert(inner_path_cloned, contents);
            } else {
                get_files(inner_path, files.clone()).await?;
            }
        }

        Ok(())
    })
}

async fn add_node_and_dependencies(
    entity: impl ExtendType,
    graph: Arc<Mutex<petgraph::Graph<Node, (NodeIndex, NodeIndex)>>>,
    dependencies: Arc<Mutex<HashMap<NodeIndex, Vec<String>>>>,
    file: &(PathBuf, String),
) -> Result<()> {
    let mut graph = graph.lock().await;

    let entity_dependencies = entity.get_dependencies();
    let (id, name) = entity.get_id_and_name();
    let new_entity = Entity::new(
        entity_dependencies.clone(),
        entity.get_mapped_type(),
        id,
        name,
        file.0.to_owned(),
        entity.get_raw(),
    );
    let node_id = new_entity.id.clone();
    let node_index = graph.add_node(Node::new(new_entity, node_id));

    // Update dependencies.
    let mut dependencies = dependencies.lock().await;
    dependencies.insert(node_index, entity_dependencies);

    Ok(())
}

/// Parse the files, generate an AST and walk it to populate the graph.
pub async fn populate_graph_from_ast(
    dependencies: Arc<Mutex<HashMap<NodeIndex, Vec<String>>>>,
    files: Arc<Mutex<HashMap<PathBuf, String>>>,
    graph: Arc<Mutex<petgraph::Graph<Node, (NodeIndex, NodeIndex)>>>,
) -> Result<()> {
    let files = files.lock().await;

    // Populate the nodes first.
    for file in files.clone() {
        let ast = parse_schema::<String>(file.1.as_str())?;

        // Reference: http://spec.graphql.org/draft/
        for definition in ast.definitions {
            let graph = graph.clone();
            let dependencies = dependencies.clone();

            match definition {
                schema::Definition::TypeDefinition(type_definition) => match type_definition {
                    schema::TypeDefinition::Enum(enum_type) => {
                        add_node_and_dependencies(enum_type, graph, dependencies, &file).await?
                    }

                    schema::TypeDefinition::InputObject(input_object_type) => {
                        add_node_and_dependencies(input_object_type, graph, dependencies, &file)
                            .await?
                    }

                    schema::TypeDefinition::Interface(interface_type) => {
                        add_node_and_dependencies(interface_type, graph, dependencies, &file)
                            .await?
                    }

                    schema::TypeDefinition::Object(object_type) => {
                        add_node_and_dependencies(object_type, graph, dependencies, &file).await?
                    }

                    schema::TypeDefinition::Scalar(scalar_type) => {
                        add_node_and_dependencies(scalar_type, graph, dependencies, &file).await?
                    }

                    schema::TypeDefinition::Union(union_type) => {
                        add_node_and_dependencies(union_type, graph, dependencies, &file).await?
                    }
                },

                schema::Definition::SchemaDefinition(schema_definition) => {
                    add_node_and_dependencies(schema_definition, graph, dependencies, &file).await?
                }

                schema::Definition::DirectiveDefinition(directive_definition) => {
                    add_node_and_dependencies(directive_definition, graph, dependencies, &file)
                        .await?
                }

                schema::Definition::TypeExtension(type_extension) => {
                    match type_extension {
                        schema::TypeExtension::Object(object_type_extension) => {
                            add_node_and_dependencies(
                                object_type_extension,
                                graph,
                                dependencies,
                                &file,
                            )
                            .await?
                        }

                        schema::TypeExtension::Scalar(scalar_type_extension) => {
                            add_node_and_dependencies(
                                scalar_type_extension,
                                graph,
                                dependencies,
                                &file,
                            )
                            .await?
                        }

                        schema::TypeExtension::Interface(interface_type_extension) => {
                            add_node_and_dependencies(
                                interface_type_extension,
                                graph,
                                dependencies,
                                &file,
                            )
                            .await?
                        }

                        schema::TypeExtension::Union(union_type_extension) => {
                            add_node_and_dependencies(
                                union_type_extension,
                                graph,
                                dependencies,
                                &file,
                            )
                            .await?
                        }

                        schema::TypeExtension::Enum(enum_type_extension) => {
                            add_node_and_dependencies(
                                enum_type_extension,
                                graph,
                                dependencies,
                                &file,
                            )
                            .await?
                        }

                        schema::TypeExtension::InputObject(input_object_type_extension) => {
                            add_node_and_dependencies(
                                input_object_type_extension,
                                graph,
                                dependencies,
                                &file,
                            )
                            .await?
                        }
                    };
                }
            }
        }
    }

    // Populate the edges.
    let dependencies = &*dependencies.lock().await;

    for (node_index, inner_dependencies) in dependencies {
        for dependency in inner_dependencies {
            let mut graph = graph.lock().await;
            // https://docs.rs/petgraph/0.5.1/petgraph/graph/struct.Graph.html#method.node_indices
            if let Some(index) = *&graph
                .node_indices()
                .find(|index| graph[*index].id == *dependency)
            {
                match &graph[node_index.clone()].entity.graphql {
                    // Reverse edge for extension types.
                    GraphQL::TypeExtension(GraphQLType::Enum)
                    | GraphQL::TypeExtension(GraphQLType::InputObject)
                    | GraphQL::TypeExtension(GraphQLType::Interface)
                    | GraphQL::TypeExtension(GraphQLType::Object)
                    | GraphQL::TypeExtension(GraphQLType::Scalar)
                    | GraphQL::TypeExtension(GraphQLType::Union) => {
                        &graph.update_edge(node_index.clone(), index, (node_index.clone(), index));
                    }
                    _ => {
                        &graph.update_edge(index, node_index.clone(), (index, node_index.clone()));
                    }
                };
            }
        }
    }

    Ok(())
}
