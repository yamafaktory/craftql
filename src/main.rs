mod config;
mod state;
mod utils;

use crate::{
    state::State,
    utils::{foo, get_files},
};

use anyhow::Result;
use clap::{crate_authors, crate_description, crate_version, Clap};

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

    // Walk the GraphQL files and populate the data.
    get_files(opts.path, shared_data).await?;

    // TODO
    foo(shared_data_cloned).await?;

    Ok(())
}
