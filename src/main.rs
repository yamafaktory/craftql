mod config;
mod state;
mod utils;

use crate::{
    state::State,
    utils::{create_graph, get_files},
};

use anyhow::Result;
use clap::{crate_authors, crate_description, crate_version, Clap};
use petgraph::dot::Dot;

#[derive(Clap)]
#[clap(author = crate_authors!(), about = crate_description!(), version = crate_version!())]
struct Opts {
    path: String,
}

#[async_std::main]
async fn main() -> Result<()> {
    let opts: Opts = Opts::parse();
    let state = State::new();
    let shared_data = state.shared;
    let shared_data_cloned = shared_data.clone();
    let shared_data_cloned_cloned = shared_data.clone();

    // Walk the GraphQL files and populate the data.
    get_files(opts.path, shared_data).await?;

    // TODO
    create_graph(shared_data_cloned).await?;

    let data = shared_data_cloned_cloned.lock().await;
    println!("{:?}", Dot::new(&data.graph));

    Ok(())
}
