//! # CraftQL
//! TODO

#![warn(missing_debug_implementations, missing_docs)]
#![deny(nonstandard_style)]

mod config;
mod extend_types;
mod state;
mod utils;

use crate::{
    state::State,
    utils::{find_node, get_files, populate_graph_from_ast},
};

use anyhow::Result;
use async_std::path::PathBuf;
use clap::{crate_authors, crate_description, crate_version, Clap};
use petgraph::dot::{Config, Dot};

#[derive(Clap)]
#[clap(author = crate_authors!(), about = crate_description!(), version = crate_version!())]
struct Opts {
    // Path to get files from.
    path: PathBuf,
    // Get one node.
    #[clap(short, long)]
    node: Option<String>,
}

#[async_std::main]
async fn main() -> Result<()> {
    let opts: Opts = Opts::parse();
    let state = State::new();
    let shared_data = state.shared;
    let shared_data_for_populate = shared_data.clone();

    // Walk the GraphQL files and populate the data.
    get_files(opts.path, shared_data.files).await?;

    // Populate the graph
    populate_graph_from_ast(
        shared_data_for_populate.dependencies,
        shared_data_for_populate.files,
        shared_data_for_populate.graph,
    )
    .await?;

    if let Some(node) = opts.node {
        find_node(node, shared_data.graph.clone()).await?;

        return Ok(());
    }

    // Render the graph without edges.
    let graph = &*shared_data.graph.lock().await;
    println!("{:?}", Dot::with_config(&graph, &[Config::EdgeNoLabel]));

    // TODO:
    // What should be the default? Getting a graph?
    // - flag to get orphans
    // - flag to get graph?
    // - flag to output content of one entity
    // - flag to list dependencies
    // - tests
    // - flag to find an entity by name

    Ok(())
}
