use async_std::sync::{Arc, Mutex};
use std::collections::HashMap;

#[derive(Debug)]
pub struct State {
    pub shared: Arc<Mutex<Data>>,
}

#[derive(Debug)]
pub struct Data {
    pub files: HashMap<String, String>,
}

impl State {
    pub fn new() -> Self {
        State {
            shared: Arc::new(Mutex::new(Data {
                files: HashMap::new(),
            })),
        }
    }
}
