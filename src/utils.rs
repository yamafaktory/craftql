use crate::{
    config::ALLOWED_EXTENSIONS,
    extend_types::ExtendType,
    state::{Entity, GraphQL, GraphQLType, Node},
};

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
use petgraph::{graph::NodeIndex, Direction};
use std::{collections::HashMap, process::exit};

/// Check if a file extension is allowed.
fn is_extension_allowed(extension: &str) -> bool {
    ALLOWED_EXTENSIONS.to_vec().contains(&extension)
}

/// Print missing definitions.
pub async fn print_missing_definitions(
    graph: Arc<Mutex<petgraph::Graph<Node, (NodeIndex, NodeIndex)>>>,
    missing_definitions: Arc<Mutex<HashMap<NodeIndex, Vec<String>>>>,
) -> Result<()> {
    let graph = graph.lock().await;
    let missing_definitions = missing_definitions.lock().await;

    for (node_index, definitions) in missing_definitions.iter() {
        println!(
            "\n# {} {} not defined in:{}",
            definitions.join(", "),
            if definitions.len() == 1 { "is" } else { "are" },
            graph.node_weight(*node_index).unwrap().entity,
        );
    }

    Ok(())
}

/// Find and return neighbors of a node.
pub async fn find_neighbors(
    node: &str,
    graph: Arc<Mutex<petgraph::Graph<Node, (NodeIndex, NodeIndex)>>>,
    direction: Direction,
) -> Vec<Entity> {
    let graph = graph.lock().await;

    match graph.node_indices().find(|index| graph[*index].id == node) {
        Some(index) => graph
            .neighbors_directed(index, direction)
            .map(|index| &graph.node_weight(index).unwrap().entity)
            .cloned()
            .collect::<Vec<Entity>>(),
        None => vec![],
    }
}

/// Print orphan nodes.
pub async fn find_and_print_neighbors(
    node: &str,
    graph: Arc<Mutex<petgraph::Graph<Node, (NodeIndex, NodeIndex)>>>,
    direction: Direction,
) -> Result<()> {
    let graph_clone = graph.clone();

    // Ensure that the node exists!
    find_node(node, graph).await?;

    let dependencies = find_neighbors(node, graph_clone, direction).await;

    if dependencies.is_empty() {
        eprintln!("No dependencies found for node {}", node);
        exit(1);
    }

    for dependency in dependencies {
        println!("{}", dependency);
    }

    Ok(())
}

/// Find and return orphan nodes.
pub async fn find_orphans(
    graph: Arc<Mutex<petgraph::Graph<Node, (NodeIndex, NodeIndex)>>>,
) -> Vec<Entity> {
    let graph = &graph.lock().await;
    let externals = graph.externals(Direction::Outgoing);
    let has_root_schema = graph
        .node_indices()
        .any(|index| graph[index].id == "schema");

    externals
        .filter_map(|index| {
            let entity = graph.node_weight(index).unwrap().entity.clone();

            match entity.graphql {
                // Skip root schema has it can't have outgoing edges.
                GraphQL::Schema => None,
                // Skip Mutation, Query and Subscription if no root schema is defined
                // as those nodes can't have outgoing edges.
                GraphQL::TypeDefinition(GraphQLType::Object)
                    if (!has_root_schema
                        && (entity.name == "Mutation"
                            || entity.name == "Query"
                            || entity.name == "Subscription")) =>
                {
                    None
                }
                _ => Some(entity),
            }
        })
        .collect::<Vec<Entity>>()
}

/// Print orphan nodes.
pub async fn find_and_print_orphans(
    graph: Arc<Mutex<petgraph::Graph<Node, (NodeIndex, NodeIndex)>>>,
) -> Result<()> {
    let orphans = find_orphans(graph).await;

    if orphans.is_empty() {
        eprintln!("No orphan node found");
        exit(1);
    }

    for orphan in orphans {
        println!("{}", orphan);
    }

    Ok(())
}

/// Find a node by name, display it with syntax highlighting or exit.
pub async fn find_node(
    node: &str,
    graph: Arc<Mutex<petgraph::Graph<Node, (NodeIndex, NodeIndex)>>>,
) -> Result<()> {
    let graph = graph.lock().await;

    match graph.node_indices().find(|index| graph[*index].id == node) {
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
    filter: &[GraphQL],
    graph: Arc<Mutex<petgraph::Graph<Node, (NodeIndex, NodeIndex)>>>,
    dependencies: Arc<Mutex<HashMap<NodeIndex, Vec<String>>>>,
    file: &(PathBuf, String),
) -> Result<()> {
    // If a filter is provided and the mapped type of the entity is not part of
    // this filter, skip it.
    if !filter.is_empty() && !filter.to_vec().contains(&entity.get_mapped_type()) {
        return Ok(());
    }

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
    filter: &[GraphQL],
    graph: Arc<Mutex<petgraph::Graph<Node, (NodeIndex, NodeIndex)>>>,
    missing_definitions: Arc<Mutex<HashMap<NodeIndex, Vec<String>>>>,
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
                schema::Definition::TypeDefinition(type_definition) => {
                    add_node_and_dependencies(type_definition, filter, graph, dependencies, &file)
                        .await?
                }
                schema::Definition::TypeExtension(type_extension) => {
                    add_node_and_dependencies(type_extension, filter, graph, dependencies, &file)
                        .await?
                }
                schema::Definition::SchemaDefinition(schema_definition) => {
                    add_node_and_dependencies(schema_definition, filter, graph, dependencies, &file)
                        .await?
                }
                schema::Definition::DirectiveDefinition(directive_definition) => {
                    add_node_and_dependencies(
                        directive_definition,
                        filter,
                        graph,
                        dependencies,
                        &file,
                    )
                    .await?
                }
            }
        }
    }

    // Populate the edges.
    let dependencies = &*dependencies.lock().await;

    for (node_index, inner_dependencies) in dependencies {
        let mut node_missing_definitions: Vec<String> = vec![];

        for dependency in inner_dependencies {
            let mut graph = graph.lock().await;

            match graph
                .node_indices()
                .find(|index| graph[*index].id == *dependency)
            {
                Some(index) => match &graph[*node_index].entity.graphql {
                    // Reverse edge for extension types.
                    GraphQL::TypeExtension(GraphQLType::Enum)
                    | GraphQL::TypeExtension(GraphQLType::InputObject)
                    | GraphQL::TypeExtension(GraphQLType::Interface)
                    | GraphQL::TypeExtension(GraphQLType::Object)
                    | GraphQL::TypeExtension(GraphQLType::Scalar)
                    | GraphQL::TypeExtension(GraphQLType::Union) => {
                        graph.update_edge(*node_index, index, (*node_index, index));
                    }
                    _ => {
                        graph.update_edge(index, *node_index, (index, *node_index));
                    }
                },
                None => match dependency.as_str() {
                    // Built-in Scalars, skip.
                    "Boolean" | "Float" | "ID" | "Int" | "String" => {}
                    // Keep track of possible missing definitions, should have been resolved at this point!
                    _ => {
                        node_missing_definitions.push(dependency.to_owned());
                    }
                },
            }
        }

        if !node_missing_definitions.is_empty() {
            missing_definitions
                .lock()
                .await
                .insert(*node_index, node_missing_definitions);
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::state::{Data, GraphQL, GraphQLType, State};

    use async_std::task;
    use petgraph::graph::NodeIndex;

    async fn scaffold(files: Vec<(PathBuf, String)>, filters: &[GraphQL]) -> Data {
        let state = State::new();
        let shared_data = state.shared;
        let shared_data_for_populate = shared_data.clone();

        task::block_on(async {
            let mut shared_files = shared_data.files.lock().await;

            for (path, contents) in files {
                shared_files.insert(path, contents);
            }
        });

        populate_graph_from_ast(
            shared_data_for_populate.dependencies,
            shared_data_for_populate.files,
            filters,
            shared_data_for_populate.graph,
            shared_data_for_populate.missing_definitions,
        )
        .await
        .unwrap();

        shared_data
    }

    #[async_std::test]
    async fn check_dependencies_and_graph() {
        let house_contents = "type House { price: Int! rooms: Int! @test owner: Owner! }";
        let house_dependencies = vec!["@test", "Int", "Owner"];
        let house_name = "House";
        let house_path = "some_path/House.gql";

        let owner_contents = "type Owner { name: String! }";
        let owner_dependencies = vec!["String"];
        let owner_name = "Owner";
        let owner_path = "some_path/Owner.graphql";

        let shared_data = scaffold(
            vec![
                (PathBuf::from(house_path), String::from(house_contents)),
                (PathBuf::from(owner_path), String::from(owner_contents)),
            ],
            &[],
        )
        .await;

        let dependencies = shared_data.dependencies.lock().await;

        assert_eq!(dependencies.len(), 2);

        // There no determined insertion order, assign dependencies lists based
        // on respective assumed lengths.
        let (current_owner_dependencies, current_house_dependencies, is_same_order) =
            if dependencies.get(&NodeIndex::new(0)).unwrap().len() == house_dependencies.len() {
                (
                    dependencies.get(&NodeIndex::new(1)).unwrap(),
                    dependencies.get(&NodeIndex::new(0)).unwrap(),
                    true,
                )
            } else {
                (
                    dependencies.get(&NodeIndex::new(0)).unwrap(),
                    dependencies.get(&NodeIndex::new(1)).unwrap(),
                    false,
                )
            };

        // List of dependencies should match.
        assert_eq!(current_owner_dependencies, &owner_dependencies);
        assert_eq!(current_house_dependencies, &house_dependencies);

        // Graph should contains 2 nodes and 1 edge.
        let graph = shared_data.graph.lock().await;
        assert_eq!(graph.node_count(), 2);
        assert_eq!(graph.edge_count(), 1);

        // Check house.
        let house_node_index = if is_same_order {
            NodeIndex::new(0)
        } else {
            NodeIndex::new(1)
        };
        let house = graph.node_weight(house_node_index).unwrap();
        assert_eq!(house.id, String::from(house_name));
        assert_eq!(house.entity.dependencies, house_dependencies);
        assert_eq!(
            house.entity.graphql,
            GraphQL::TypeDefinition(GraphQLType::Object)
        );
        assert_eq!(house.entity.id, String::from(house_name));
        assert_eq!(house.entity.name, String::from(house_name));
        assert_eq!(house.entity.path, PathBuf::from(house_path));
        // We need to parse and format the AST in order to get the same output!
        assert_eq!(
            house.entity.raw.to_string(),
            format!(
                "{}",
                parse_schema::<String>(house_contents).unwrap().to_owned()
            )
        );

        // Check owner.
        let owner_node_index = if is_same_order {
            NodeIndex::new(1)
        } else {
            NodeIndex::new(0)
        };
        let owner = graph.node_weight(owner_node_index).unwrap();
        assert_eq!(owner.id, String::from(owner_name));
        assert_eq!(owner.entity.dependencies, owner_dependencies);
        assert_eq!(
            owner.entity.graphql,
            GraphQL::TypeDefinition(GraphQLType::Object)
        );
        assert_eq!(owner.entity.id, String::from(owner_name));
        assert_eq!(owner.entity.name, String::from(owner_name));
        assert_eq!(owner.entity.path, PathBuf::from(owner_path));
        // We need to parse and format the AST in order to get the same output!
        assert_eq!(
            owner.entity.raw.to_string(),
            format!(
                "{}",
                parse_schema::<String>(owner_contents).unwrap().to_owned()
            )
        );

        // Check the edges. Owner should be directed to House, not the other
        // way around!
        assert!(graph.contains_edge(owner_node_index, house_node_index));
        assert!(!graph.contains_edge(house_node_index, owner_node_index));
    }

    #[async_std::test]
    async fn check_orphans() {
        let shared_data = scaffold(
            vec![(
                PathBuf::from("some_path/Peer.gql"),
                String::from("type Peer { id: String! }"),
            )],
            &[],
        )
        .await;

        task::block_on(async {
            let graph = shared_data.graph.lock().await;
            assert_eq!(graph.node_count(), 1);
            assert_eq!(graph.edge_count(), 0);
        });

        assert_eq!(find_orphans(shared_data.graph).await.len(), 1);
    }

    #[async_std::test]
    async fn check_orphans_with_schema() {
        let shared_data = scaffold(
            vec![(
                PathBuf::from("some_path/Schema.gql"),
                String::from("schema { query: Query }"),
            )],
            &[],
        )
        .await;

        task::block_on(async {
            let graph = shared_data.graph.lock().await;
            assert_eq!(graph.node_count(), 1);
            assert_eq!(graph.edge_count(), 0);
        });

        assert_eq!(find_orphans(shared_data.graph).await.len(), 0);
    }

    #[async_std::test]
    async fn check_orphans_without_schema_but_with_query_mutation_subscription() {
        let shared_data = scaffold(
            vec![
                (
                    PathBuf::from("some_path/Query.gql"),
                    String::from("type Query { foo(bar: String): String }"),
                ),
                (
                    PathBuf::from("some_path/Mutation.gql"),
                    String::from("type Mutation { foo(bar: String): String }"),
                ),
                (
                    PathBuf::from("some_path/Subscription.gql"),
                    String::from("type Subscription { foo(bar: String): String }"),
                ),
            ],
            &[],
        )
        .await;

        task::block_on(async {
            let graph = shared_data.graph.lock().await;
            assert_eq!(graph.node_count(), 3);
            assert_eq!(graph.edge_count(), 0);
        });

        assert_eq!(find_orphans(shared_data.graph).await.len(), 0);
    }

    #[async_std::test]
    async fn check_neighbors() {
        let shared_data = scaffold(
            vec![
                (
                    PathBuf::from("some_path/Foo.gql"),
                    String::from("type Foo { field: Bar }"),
                ),
                (
                    PathBuf::from("some_path/Bar.gql"),
                    String::from("interface Bar { id: ID!}"),
                ),
            ],
            &[],
        )
        .await;

        task::block_on(async {
            let graph = shared_data.graph.lock().await;
            assert_eq!(graph.node_count(), 2);
            assert_eq!(graph.edge_count(), 1);
        });

        // Foo depends on Bar but is not a dependency.
        let incoming = find_neighbors("Foo", shared_data.graph.clone(), Direction::Incoming).await;
        let outgoing = find_neighbors("Foo", shared_data.graph.clone(), Direction::Outgoing).await;

        assert_eq!(incoming.len(), 1);
        assert_eq!(incoming.first().unwrap().name, "Bar");
        assert_eq!(outgoing.len(), 0);

        // Bar depends on nothing but is a dependency of Foo.
        let incoming = find_neighbors("Bar", shared_data.graph.clone(), Direction::Incoming).await;
        let outgoing = find_neighbors("Bar", shared_data.graph.clone(), Direction::Outgoing).await;

        assert_eq!(incoming.len(), 0);
        assert_eq!(outgoing.len(), 1);
        assert_eq!(outgoing.first().unwrap().name, "Foo");
    }

    #[async_std::test]
    async fn check_missing_definitions() {
        let shared_data = scaffold(
            vec![
                (
                    PathBuf::from("some_path/Foo.gql"),
                    String::from("type Foo { field: Woot, otherField: Why }"),
                ),
                (
                    PathBuf::from("some_path/Bar.gql"),
                    String::from("interface Bar { id: What!}"),
                ),
            ],
            &[],
        )
        .await;

        let graph = shared_data.graph.lock().await;
        assert_eq!(graph.node_count(), 2);
        assert_eq!(graph.edge_count(), 0);

        let missing_definitions = shared_data.missing_definitions.lock().await;

        let (bar_missing_dependencies, foo_missing_dependencies) =
            if missing_definitions.get(&NodeIndex::new(0)).unwrap().len() == 2 {
                (
                    missing_definitions.get(&NodeIndex::new(1)).unwrap(),
                    missing_definitions.get(&NodeIndex::new(0)).unwrap(),
                )
            } else {
                (
                    missing_definitions.get(&NodeIndex::new(0)).unwrap(),
                    missing_definitions.get(&NodeIndex::new(1)).unwrap(),
                )
            };

        assert_eq!(
            *foo_missing_dependencies,
            vec![String::from("Why"), String::from("Woot")]
        );
        assert_eq!(*bar_missing_dependencies, vec![String::from("What")]);
    }

    #[async_std::test]
    async fn check_filtering() {
        let shared_data = scaffold(
            vec![
                (
                    PathBuf::from("some_path/Foo.gql"),
                    String::from("type Foo { a: ID! }"),
                ),
                (
                    PathBuf::from("some_path/Bar.gql"),
                    String::from("interface Bar { b: ID! }"),
                ),
                (
                    PathBuf::from("some_path/Cow.gql"),
                    String::from("input Cow { c: ID! }"),
                ),
            ],
            &[
                GraphQL::TypeDefinition(GraphQLType::Object),
                GraphQL::TypeDefinition(GraphQLType::InputObject),
            ],
        )
        .await;

        let graph = shared_data.graph.lock().await;
        assert_eq!(graph.node_count(), 2);
        assert_eq!(graph.edge_count(), 0);

        let mut selected_entities = graph
            .node_indices()
            .map(|index| &graph.node_weight(index).unwrap().id)
            .collect::<Vec<&String>>();
        // Sort the entities as the order is not as there no determined insertion order.
        selected_entities.sort();
        assert_eq!(selected_entities, vec!["Cow", "Foo"]);
    }
}
