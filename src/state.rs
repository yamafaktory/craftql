use async_std::sync::{Arc, Mutex};
use petgraph::{graph::NodeIndex, Graph};
use std::{collections::HashMap, fmt};

#[derive(Debug)]
pub struct State {
    pub shared: Arc<Mutex<Data>>,
}

#[derive(Debug)]
pub enum GraphQLDefinition {
    Definition,
    Schema,
}

#[derive(Debug)]
/// Derived from graphql_parser::schema::TypeDefinition enum.
pub enum GraphQLType<D = GraphQLDefinition> {
    Definition(D),
    Enum,
    InputObject,
    Interface,
    Object,
    Scalar,
    Union,
}

/// Represents a GraphQL entity.
pub struct Entity {
    dependencies: Vec<String>,
    graphql_type: GraphQLType,
    name: String,
    path: String,
    raw: String,
}

impl Entity {
    pub fn new(
        dependencies: Vec<String>,
        graphql_type: GraphQLType,
        name: String,
        path: String,
        raw: String,
    ) -> Self {
        Entity {
            dependencies,
            graphql_type,
            name,
            path,
            raw,
        }
    }
}

impl fmt::Debug for Entity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "name: {}, type: {:?}, dependencies: {:?}",
            self.name, self.graphql_type, self.dependencies
        )
    }
}

pub struct Node {
    pub entity: Entity,
    // Using the entity name as id is safe as it is unique.
    // http://spec.graphql.org/draft/#sec-Schema
    pub id: String,
}

impl Node {
    pub fn new(entity: Entity, id: String) -> Self {
        Node { entity, id }
    }
}

impl fmt::Debug for Node {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "entity: {:?}", self.entity)
    }
}

#[derive(Debug)]
pub struct Data {
    pub files: HashMap<String, String>,
    pub graph: petgraph::Graph<Node, (NodeIndex, NodeIndex)>,
}

impl State {
    pub fn new() -> Self {
        State {
            shared: Arc::new(Mutex::new(Data {
                files: HashMap::new(),
                graph: Graph::<Node, (NodeIndex, NodeIndex)>::new(),
            })),
        }
    }
}
