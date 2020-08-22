use async_std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};
use petgraph::{graph::NodeIndex, Graph};
use std::{collections::HashMap, fmt};

#[derive(Debug)]
pub struct State {
    pub shared: Arc<Mutex<Data>>,
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
    raw: String,
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

#[derive(Debug)]
pub struct Data {
    pub files: HashMap<PathBuf, String>,
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
