//! # CraftQL
//! TODO

#![forbid(rust_2018_idioms)]
#![warn(missing_debug_implementations, missing_docs)]
#![deny(unsafe_code, nonstandard_style)]

mod config;
mod extend_types;
mod state;
mod utils;

use crate::{
    state::State,
    utils::{
        find_and_print_neighbors, find_and_print_orphans, find_node, get_files,
        populate_graph_from_ast,
    },
};

use anyhow::Result;
use async_std::path::PathBuf;
use clap::{crate_authors, crate_description, crate_version, Clap};
use petgraph::dot::{Config, Dot};
use petgraph::Direction;

#[derive(Clap)]
#[clap(author = crate_authors!(), about = crate_description!(), version = crate_version!())]
struct Opts {
    /// Path to get files from
    path: PathBuf,
    /// Finds and displays incoming dependencies of a node
    #[clap(short, long)]
    incoming_dependencies: Option<String>,
    /// Finds and displays orphan(s) node(s)
    #[clap(short = "O", long)]
    orphans: bool,
    /// Finds and displays outgoing dependencies of a node
    #[clap(short, long)]
    outgoing_dependencies: Option<String>,
    /// Finds and displays one node
    #[clap(short, long)]
    node: Option<String>,
    /// Finds and displays multiple nodes
    #[clap(short = "N", long)]
    nodes: Vec<String>,
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
        shared_data.dependencies,
        shared_data_for_populate.files,
        shared_data_for_populate.graph,
    )
    .await?;

    if let Some(ref node) = opts.incoming_dependencies {
        find_and_print_neighbors(node, shared_data.graph.clone(), Direction::Incoming).await?;

        return Ok(());
    }

    if let Some(ref node) = opts.outgoing_dependencies {
        find_and_print_neighbors(node, shared_data.graph.clone(), Direction::Outgoing).await?;

        return Ok(());
    }

    if let Some(ref node) = opts.node {
        find_node(node, shared_data.graph.clone()).await?;

        return Ok(());
    }

    if !opts.nodes.is_empty() {
        for ref node in opts.nodes {
            find_node(node, shared_data.graph.clone()).await?;
        }

        return Ok(());
    }

    if opts.orphans {
        find_and_print_orphans(shared_data.graph.clone()).await?;

        return Ok(());
    }

    // Render the graph without edges.
    let graph = &*shared_data.graph.lock().await;
    println!("\n{:?}", Dot::with_config(&graph, &[Config::EdgeNoLabel]));

    Ok(())
}
