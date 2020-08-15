use async_std::sync::{Arc, Mutex};
use petgraph::Graph;
use std::{collections::HashMap, fmt};

#[derive(Debug)]
pub struct State {
    pub shared: Arc<Mutex<Data>>,
}

#[derive(Debug)]
/// Derived from graphql_parser::schema::TypeDefinition enum.
pub enum GraphQLType {
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
    pub id: String,
}

impl Node {
    pub fn new(entity: Entity, id: String) -> Self {
        Node { entity, id }
    }
}

impl fmt::Debug for Node {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "id: {}, entity: {:?}", self.id, self.entity)
    }
}

#[derive(Debug)]
pub struct Data<E = ()> {
    pub files: HashMap<String, String>,
    pub graph: petgraph::Graph<Node, E>,
}

impl State {
    pub fn new() -> Self {
        State {
            shared: Arc::new(Mutex::new(Data {
                files: HashMap::new(),
                graph: Graph::<Node, ()>::new(),
            })),
        }
    }
}
