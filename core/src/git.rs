use anyhow::anyhow;

pub struct GitUpdate {
    pub from_commit: String,
    pub to_commit: String,
}

#[derive(Debug)]
pub struct Git {
    exe: String,
    pub repo: String,
}

impl Git {
    pub fn new(repo: String) -> Self {
        Self {
            exe: "git".to_string(),
            repo,
        }
    }

    pub fn update(&self) -> anyhow::Result<GitUpdate> {
        let mut git = std::process::Command::new(&self.exe);
        git.args(["pull"]);
        git.current_dir(&self.repo);
        let a = git.output()?;
        let output = String::from_utf8(a.stdout)?;
        let Some(first_line) = output.lines().nth(0) else {
            return Err(anyhow::anyhow!("No output lines"));
        };
        let Some(words) = first_line.split(' ').next_back() else {
            return Err(anyhow::anyhow!("Empty line"));
        };

        let Some((old, new)) = words.split_once("..") else {
            return Err(anyhow!("Already update"));
        };

        Ok(GitUpdate {
            from_commit: old.to_string(),
            to_commit: new.to_string(),
        })
    }

    pub fn diff(&self, from_commit: &str, to_commit: &str) -> Option<String> {
        let mut git = std::process::Command::new("git");
        git.args([
            "--no-pager",
            "diff",
            "-U1",
            "--no-color",
            &format!("{from_commit}..{to_commit}"),
        ]);
        git.current_dir(&self.repo);
        let a = git.output().ok()?;
        String::from_utf8(a.stdout).ok()
    }

    pub fn checkout(&self, commit: &str) -> anyhow::Result<()> {
        let mut git = std::process::Command::new("git");
        git.args(["--no-pager", "checkout", commit]);
        git.current_dir(&self.repo);
        git.status()?;
        Ok(())
    }
}
