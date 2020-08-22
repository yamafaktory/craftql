mod config;
mod state;
mod utils;

use crate::{
    state::State,
    utils::{get_files, populate_graph_from_ast},
};

use anyhow::Result;
use async_std::path::PathBuf;
use clap::{crate_authors, crate_description, crate_version, Clap};
use petgraph::dot::{Config, Dot};

#[derive(Clap)]
#[clap(author = crate_authors!(), about = crate_description!(), version = crate_version!())]
struct Opts {
    path: PathBuf,
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

    // Populate the graph
    populate_graph_from_ast(shared_data_cloned).await?;

    let data = shared_data_cloned_cloned.lock().await;
    println!(
        "{:?}",
        Dot::with_config(&data.graph, &[Config::EdgeNoLabel])
    );

    // TODO:
    // What should be the default? Getting a graph?
    // - flag to get orphans
    // - flag to get graph?
    // - flag to output content of one entity
    // - flag to list dependencies
    // - tests

    Ok(())
}
