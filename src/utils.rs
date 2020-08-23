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
use petgraph::graph::NodeIndex;
use std::collections::HashMap;

fn is_extension_allowed(extension: &str) -> bool {
    ALLOWED_EXTENSIONS.to_vec().contains(&extension)
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

        if file_type.is_file() {
            if is_extension_allowed(
                Path::new(thread_safe_path.as_ref())
                    .extension()
                    .unwrap()
                    .to_str()
                    .unwrap(),
            ) {
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

            if !is_dir && is_extension_allowed(&inner_path.extension().unwrap().to_str().unwrap()) {
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
    let entity_dependencies = entity.get_dependencies();
    let mut graph = graph.lock().await;
    let (file, contents) = file;

    let node_index = graph.add_node(Node::new(
        Entity::new(
            entity_dependencies.clone(),
            entity.get_mapped_type(), // TODO: impl on ExtendType
            entity.get_id(),
            file.to_owned(),
            contents.to_owned(),
        ),
        entity.get_id(),
    ));

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
            let maybe_index = &graph
                .node_indices()
                .find(|index| graph[*index].id == *dependency);

            if let Some(index) = *maybe_index {
                &graph.update_edge(index, node_index.clone(), (index, node_index.clone()));
            }
        }
    }

    Ok(())
}
