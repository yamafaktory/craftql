use async_std::sync::{Arc, Mutex};
use petgraph::Graph;
use std::{collections::HashMap, fmt};

#[derive(Debug)]
pub struct State {
    pub shared: Arc<Mutex<Data>>,
}

pub struct Node<N> {
    pub id: String,
    pub inner: N,
}

impl<N> fmt::Debug for Node<N>
where
    N: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "File: {} Inner: {:?}", self.id, self.inner)
    }
}

#[derive(Debug)]
pub struct Data<N = (), E = ()> {
    pub files: HashMap<String, String>,
    pub graph: petgraph::Graph<Node<N>, E>,
}

impl State {
    pub fn new() -> Self {
        State {
            shared: Arc::new(Mutex::new(Data {
                files: HashMap::new(),
                graph: Graph::<Node<()>, ()>::new(),
            })),
        }
    }
}
