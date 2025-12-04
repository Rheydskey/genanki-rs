use crate::{git::Git, tests::FakeRepo, updater::Updater};
use rstest::{fixture, rstest};

#[fixture]
pub fn get_fake_repo() -> FakeRepo {
    return FakeRepo::new();
}

#[rstest]
pub fn test_folder_diff(get_fake_repo: FakeRepo) {
    let repo_path = get_fake_repo.0.path().to_str().unwrap().to_string();
    let diff = Git::new(repo_path).diff("55974ad", "d60481d").unwrap();
    let folders = Updater::get_folder_with_diff(&diff).unwrap();
    assert!(folders.contains("basic_cards"));
}

#[rstest]
pub fn test_generation(get_fake_repo: FakeRepo) {
    let repo_path = get_fake_repo.0.path().to_str().unwrap().to_string();
    let a = Updater::new(repo_path.clone());
    let diff = Git::new(repo_path).diff("55974ad", "d60481d").unwrap();
    let decks = a
        .generate_decks_from_diff(&diff, "55974ad", "d60481d")
        .unwrap();
    assert_eq!(
        decks.get("basic_cards").unwrap().added[0].hash,
        "bc8d85bc874a2b9bc35885a8bf5169bff54a9e549edad419dd3f9f5901e62f54"
    );
}

#[rstest]
pub fn test_new_subdecks_folder_diff(get_fake_repo: FakeRepo) {
    let repo_path = get_fake_repo.0.path().to_str().unwrap().to_string();
    let diff = Git::new(repo_path).diff("d60481d", "54012ee").unwrap();
    let folders = Updater::get_folder_with_diff(&diff).unwrap();
    assert!(folders.contains("basic_cards/subdecks"));
}

#[rstest]
pub fn test_new_subdecks_diff_output(get_fake_repo: FakeRepo) {
    let repo_path = get_fake_repo.0.path().to_str().unwrap().to_string();
    let a = Updater::new(repo_path.clone());
    let diff = Git::new(repo_path).diff("d60481d", "54012ee").unwrap();
    let g = a
        .generate_decks_from_diff(&diff, "d60481d", "54012ee")
        .unwrap();

    eprintln!("{g:?}");

    assert_eq!(
        g.get("basic_cards::subdecks").unwrap().added[0].hash,
        "87588180b9688dab251cccca1ab23c377ea998e21a158e250772a5b770b1e098"
    );
}
