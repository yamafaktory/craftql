use async_std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};
use petgraph::{graph::NodeIndex, Graph};
use std::{collections::HashMap, fmt};

#[derive(Debug)]
pub struct State {
    pub shared: Data,
}

/// Core GraphQL types used for definitions and extensions.
#[derive(Debug, Copy, Clone)]
pub enum GraphQLType {
    Enum,
    InputObject,
    Interface,
    Object,
    Scalar,
    Union,
}

/// Derived and simplified from graphql_parser::schema enums.
pub enum GraphQL<T = GraphQLType> {
    Directive,
    Schema,
    TypeDefinition(T),
    TypeExtension(T),
}

impl<T> fmt::Debug for GraphQL<T>
where
    T: fmt::Debug + Copy,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            GraphQL::Directive => write!(f, "Directive"),
            GraphQL::Schema => write!(f, "Schema"),
            GraphQL::TypeDefinition(graphql_type) => write!(f, "{:?}", graphql_type),
            GraphQL::TypeExtension(graphql_type) => write!(f, "{:?} extension", graphql_type),
        }
    }
}

/// Represents a GraphQL entity.
pub struct Entity {
    dependencies: Vec<String>,
    graphql: GraphQL,
    name: String,
    path: PathBuf,
    pub raw: String,
}

impl Entity {
    pub fn new(
        dependencies: Vec<String>,
        graphql: GraphQL,
        name: String,
        path: PathBuf,
        raw: String,
    ) -> Self {
        Entity {
            dependencies,
            graphql,
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
            "{} ({:?}) {:?}",
            self.name, self.graphql, self.dependencies
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
        write!(f, "{:?}", self.entity)
    }
}

#[derive(Debug, Clone)]
pub struct Data {
    // Keep track of the dependencies for edges.
    pub dependencies: Arc<Mutex<HashMap<NodeIndex, Vec<String>>>>,
    pub files: Arc<Mutex<HashMap<PathBuf, String>>>,
    pub graph: Arc<Mutex<petgraph::Graph<Node, (NodeIndex, NodeIndex)>>>,
}

impl State {
    pub fn new() -> Self {
        State {
            shared: Data {
                dependencies: Arc::new(Mutex::new(HashMap::new())),
                files: Arc::new(Mutex::new(HashMap::new())),
                graph: Arc::new(Mutex::new(Graph::<Node, (NodeIndex, NodeIndex)>::new())),
            },
        }
    }
}
