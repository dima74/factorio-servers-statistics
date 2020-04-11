use crate::external_storage;

#[test]
fn test_equal_after_serialize_deserialize() {
    dotenv::dotenv().ok();

    let state_before = external_storage::fetch_state();

    let temp_file = std::env::temp_dir().join("temp.bin");
    let temp_file = temp_file.to_str().unwrap();
    external_storage::save_state_to_file(state_before.deref(), temp_file);

    let state_after = external_storage::load_state_from_file(temp_file);

    let games_before = &state_before.state.games;
    let games_after = &state_after.state.games;
    assert!(games_before == games_after);
}
