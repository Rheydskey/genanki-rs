use std::path::Path;
use std::{fs, io};
use tempfile::TempDir;

mod test_generator;
mod test_subdeck;
mod test_updater;

// https://stackoverflow.com/a/65192210
fn copy_dir_all(src: impl AsRef<Path>, dst: impl AsRef<Path>) -> io::Result<()> {
    fs::create_dir_all(&dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        if ty.is_dir() {
            copy_dir_all(entry.path(), dst.as_ref().join(entry.file_name()))?;
        } else {
            fs::copy(entry.path(), dst.as_ref().join(entry.file_name()))?;
        }
    }
    Ok(())
}

struct FakeRepo(TempDir);

impl FakeRepo {
    pub fn new() -> Self {
        let dir = TempDir::new().unwrap();
        copy_dir_all("./tests/fake_repo", &dir).unwrap();

        Self(dir)
    }
}

impl AsRef<Path> for FakeRepo {
    fn as_ref(&self) -> &Path {
        return self.0.path();
    }
}
