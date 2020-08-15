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

pub struct Entity {
    fields: Vec<String>,
    name: String,
    raw: String,
    graphql_type: GraphQLType,
}

impl Entity {
    pub fn new(fields: Vec<String>, graphql_type: GraphQLType, name: String, raw: String) -> Self {
        Entity {
            fields,
            graphql_type,
            name,
            raw,
        }
    }
}

impl fmt::Debug for Entity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Name: {} | Type: {:?}", self.name, self.graphql_type)
    }
}

pub struct Node<Entity> {
    pub id: String,
    pub entity: Entity,
}

impl<N> fmt::Debug for Node<N>
where
    N: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "File: {} Entity: {:?}", self.id, self.entity)
    }
}

#[derive(Debug)]
pub struct Data<N = Entity, E = ()> {
    pub files: HashMap<String, String>,
    pub graph: petgraph::Graph<Node<N>, E>,
}

impl State {
    pub fn new() -> Self {
        State {
            shared: Arc::new(Mutex::new(Data {
                files: HashMap::new(),
                graph: Graph::<Node<Entity>, ()>::new(),
            })),
        }
    }
}
