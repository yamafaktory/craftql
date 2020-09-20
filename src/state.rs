use async_std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};
use petgraph::{graph::NodeIndex, Graph};
use std::{collections::HashMap, fmt, str::FromStr};

/// Global state.
#[derive(Debug)]
pub struct State {
    /// Shared part of the state.
    pub shared: Data,
}

/// Core GraphQL types used for definitions and extensions.
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum GraphQLType {
    /// Enum type.
    Enum,
    /// InputObject type.
    InputObject,
    /// Interface type.
    Interface,
    /// Object type.
    Object,
    /// Scalar type.
    Scalar,
    /// Union type.
    Union,
}

/// Derived and simplified from graphql_parser::schema enums.
#[derive(Clone, PartialEq)]
pub enum GraphQL<T = GraphQLType> {
    /// Directive type.
    Directive,
    /// Schema type.
    Schema,
    /// TypeDefinition type.
    TypeDefinition(T),
    /// TypeExtension type.
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

impl FromStr for GraphQL {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "directive" => Ok(GraphQL::Directive),
            "enum" => Ok(GraphQL::TypeDefinition(GraphQLType::Enum)),
            "enum_extension" => Ok(GraphQL::TypeExtension(GraphQLType::Enum)),
            "input_object" => Ok(GraphQL::TypeDefinition(GraphQLType::InputObject)),
            "input_object_extension" => Ok(GraphQL::TypeExtension(GraphQLType::InputObject)),
            "interface" => Ok(GraphQL::TypeDefinition(GraphQLType::Interface)),
            "interface_extension" => Ok(GraphQL::TypeExtension(GraphQLType::Interface)),
            "object" => Ok(GraphQL::TypeDefinition(GraphQLType::Object)),
            "object_extension" => Ok(GraphQL::TypeExtension(GraphQLType::Object)),
            "scalar" => Ok(GraphQL::TypeDefinition(GraphQLType::Scalar)),
            "scalar_extension" => Ok(GraphQL::TypeExtension(GraphQLType::Scalar)),
            "schema" => Ok(GraphQL::Schema),
            "union" => Ok(GraphQL::TypeDefinition(GraphQLType::Union)),
            "union_extension" => Ok(GraphQL::TypeExtension(GraphQLType::Union)),
            unknown => Err(format!(r#"Unknown GraphQL type provided "{}""#, unknown)),
        }
    }
}

/// Represents a GraphQL entity.
#[derive(Clone)]
pub struct Entity {
    /// Dependencies of an entity.
    pub dependencies: Vec<String>,
    /// GraphQL type of the entity.
    pub graphql: GraphQL,
    /// Id of the entity.
    pub id: String,
    /// Name of the entity.
    pub name: String,
    /// Path of the entity.
    pub path: PathBuf,
    /// Raw representation of the entity.
    pub raw: String,
}

impl Entity {
    /// Method to create a new Entity.
    pub fn new(
        dependencies: Vec<String>,
        graphql: GraphQL,
        id: Option<String>,
        name: String,
        path: PathBuf,
        raw: String,
    ) -> Self {
        Entity {
            dependencies,
            graphql,
            // If no custom id is provided, use the name.
            id: match id {
                Some(id) => id,
                None => name.clone(),
            },
            name,
            path,
            raw,
        }
    }
}

// Used in graph generation.
impl fmt::Debug for Entity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.dependencies.is_empty() {
            write!(f, "{} ({:?})", self.name, self.graphql)
        } else {
            write!(
                f,
                "{} ({:?})\n\n[{}]",
                self.name,
                self.graphql,
                self.dependencies.join(", ")
            )
        }
    }
}

// Used with flags like --node.
impl fmt::Display for Entity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "\n# {}\n{}", self.path.to_string_lossy(), self.raw)
    }
}

/// A Node containing an Entity and a unique id.
pub struct Node {
    /// Node's entity.
    pub entity: Entity,
    /// Using the entity name as id is safe as it is unique.
    /// http://spec.graphql.org/draft/#sec-Schema
    pub id: String,
}

impl Node {
    /// Method to create a new Node.
    pub fn new(entity: Entity, id: String) -> Self {
        Node { entity, id }
    }
}

impl fmt::Debug for Node {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self.entity)
    }
}

/// Data holding the thread-safe mutexes.
#[derive(Debug, Clone)]
pub struct Data {
    /// Dependencies mutex.
    pub dependencies: Arc<Mutex<HashMap<NodeIndex, Vec<String>>>>,
    /// Files mutex.
    pub files: Arc<Mutex<HashMap<PathBuf, String>>>,
    /// Graph mutex.
    pub graph: Arc<Mutex<petgraph::Graph<Node, (NodeIndex, NodeIndex)>>>,
    /// Missing definition mutex.
    pub missing_definitions: Arc<Mutex<HashMap<NodeIndex, Vec<String>>>>,
}

impl State {
    /// Method to create a new State.
    pub fn new() -> Self {
        State {
            shared: Data {
                dependencies: Arc::new(Mutex::new(HashMap::new())),
                files: Arc::new(Mutex::new(HashMap::new())),
                graph: Arc::new(Mutex::new(Graph::<Node, (NodeIndex, NodeIndex)>::new())),
                missing_definitions: Arc::new(Mutex::new(HashMap::new())),
            },
        }
    }
}

impl Default for State {
    fn default() -> Self {
        State::new()
    }
}
