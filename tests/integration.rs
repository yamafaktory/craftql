extern crate craftql;

use anyhow::Result;
use async_std::{fs, path::PathBuf};
use craftql::{state::State, utils::get_files};

#[async_std::test]
async fn check_get_files() -> Result<()> {
    let state = State::default();
    let shared_data = state.shared;
    let shared_data_cloned = shared_data.clone();

    get_files(PathBuf::from("./tests/fixtures"), shared_data.files).await?;

    let files = shared_data_cloned.files.lock().await;

    assert_eq!(files.len(), 20);

    let contents = fs::read_to_string("./tests/fixtures/Types/Enums/Episode.gql").await?;

    assert_eq!(
        files.get(&PathBuf::from("./tests/fixtures/Types/Enums/Episode.gql")),
        Some(&contents)
    );

    Ok(())
}
