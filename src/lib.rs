extern crate ini;
use ini::Ini;
use std::path::{Path, PathBuf};
mod wyagError;

/// Git Repository object
pub struct GitRepository<'a> {
    worktree: &'a str,
    gitdir: PathBuf,
    conf: Ini,
}

impl<'a> GitRepository<'a> {
    pub fn new(path: &'a str, force: bool) -> Result<GitRepository, wyagError::WyagError> {
        // Set up the gitdir
        let git_path = Path::new(path).join(".git");
        if !(force || git_path.is_dir()) {
            let serr = "Not a git path";
            return Err(wyagError::WyagError::new(serr));
        }

        // Read configuration file in .git/config
        let git_conf_path = match repo_file_path(&git_path, false, vec!["config"]) {
            Ok(p) => p,
            Err(m) => {
                return Err(wyagError::WyagError::new("Failed to find .git folder"));
            }
        };

        // Read if exists
        let mut conf = Ini::new();
        if git_conf_path.exists() {
            match Ini::load_from_file(&git_conf_path) {
                Ok(c) => conf = c,
                Err(m) => {
                    return Err(wyagError::WyagError::new_with_error(
                        "Failed to read git config file",
                        Box::new(m),
                    ));
                }
            };
        } else if !force {
            return Err(wyagError::WyagError::new("Configuration file missing"));
        }

        if !force {
            let core = conf
                .section(Some("core".to_owned()))
                .expect("Expected a section named core, but none existed");
            let repo_format_version = core.get("repositoryformatversion").expect("expected a 'repositoryformatversion' key containing a number under the [core] section, but found nothing");
            let repo_format_version: u32 = repo_format_version.parse().expect("expected 'repositoryformatversion' to contain a valid integer, found an invalid element instead.");
            if repo_format_version != 0 {
                return Err(wyagError::WyagError::new("Unsupported repo format version"));
            }
        }

        Ok(GitRepository {
            worktree: path,
            gitdir: git_path.to_path_buf(),
            conf: conf,
        })
    }

    /// Creates a new repository at `path`
    fn repo_create(path: &str) -> Result<GitRepository, wyagError::WyagError> {
        let repo = GitRepository::new(path, true)?;

        // check that repo path is either non-existant, or is an empty dir
        let p: PathBuf = PathBuf::from(repo.worktree);

        if p.exists() {
            if p.is_file() {
                return Err(wyagError::WyagError::new(
                    "Cannot create new repository, supplied path is not a directory.",
                ));
            }
            let mut iter = std::fs::read_dir(p).expect("Failed to read contents of the supplied directory. Do you have permission to view this folder?");
            if iter.any(|_| true) {
                return Err(wyagError::WyagError::new(
                    "Cannot create new repository, supplied path is not empty.",
                ));
            }
            if let Err(m) = std::fs::create_dir_all(repo.worktree) {
                return Err(wyagError::WyagError::new(
                    "failed to create work directory for supplied repository",
                ));
            }
        }

        if let Err(m) = repo_dir_gr(&repo, true, vec!["branches"]) {
            return Err(wyagError::WyagError::new(
                "Failed to create directory Branches underneath git main dir",
            ));
        }
        if let Err(m) = repo_dir_gr(&repo, true, vec!["objects"]) {
            return Err(wyagError::WyagError::new(
                "Failed to create directory objects underneath git main dir",
            ));
        }

        if let Err(m) = repo_dir_gr(&repo, true, vec!["refs", "tags"]) {
            return Err(wyagError::WyagError::new(
                "Failed to create directory refs/tags underneath git main dir",
            ));
        }

        if let Err(m) = repo_dir_gr(&repo, true, vec!["refs", "heads"]) {
            return Err(wyagError::WyagError::new(
                "Failed to create directory refs/heads underneath git main dir",
            ));
        }

        // .git/description
        match repo_file_gr(&repo, false, vec!["description"]) {
            Ok(p) => {
                if let Err(m) = std::fs::write(
                    p,
                    "Unnamed repository; edit this file 'description' to name the repository.\n",
                ) {
                    return Err(wyagError::WyagError::new("Failed writing Description file"));
                };
            }
            Err(m) => {
                return Err(wyagError::WyagError::new(
                    "Failed to create description file under git main",
                ));
            }
        };

        // .git/HEAD
        match repo_file_gr(&repo, false, vec!["HEAD"]) {
            Ok(p) => {
                if let Err(m) = std::fs::write(p, "ref: refs/heads/master\n") {
                    return Err(wyagError::WyagError::new("Failed writing HEAD file"));
                }
            }
            Err(m) => {
                return Err(wyagError::WyagError::new(
                    "Failed to create HEAD file under git main",
                ));
            }
        };

        // .git/config
        match repo_file_gr(&repo, false, vec!["config"]) {
            Ok(p) => {
                let conf = GitRepository::repo_default_config();
                conf.write_to_file(p)
                    .expect("Failed to write ini config to file");
            }
            Err(m) => {
                return Err(wyagError::WyagError::new(
                    "Failed to create config file under git main",
                ));
            }
        };

        return Ok(repo);
    }

    /// Returns an ini::Ini representation of the default {path}/.git/config file
    ///
    /// Does not write to disk.
    ///
    /// `repositoryformatversion` the version of the gitdir format. 0 means the initial format, 1 the same with extensions. If > 1, git will panic; wyag will only accept 0.
    ///
    /// `filemode = true`  disables tracking of file mode changes in the work tree.
    ///
    /// `bare = false`  indicates that this repository has a worktree. Git supports an optional `worktree` key which indicates the location of the worktree, if not `..`; wyag doesn’t.
    fn repo_default_config() -> Ini {
        let mut conf = Ini::new();
        conf.with_section(Some("core".to_owned()))
            .set("repositoryformatversion", "0")
            .set("filemode", "false")
            .set("bare", "false");
        // conf.write_to_file("conf.ini").unwrap();
        conf
    }
}

/// Compute path under the repo's gitdir using a GitRepository
fn repo_path_gr(gr: &GitRepository, paths: Vec<&str>) -> PathBuf {
    return repo_path_path(&gr.gitdir, paths);
}

/// Compute path under the repo's gitdir using a raw path as the root
fn repo_path_path(root: &PathBuf, paths: Vec<&str>) -> PathBuf {
    let mut p = root.to_path_buf();
    for fragment in paths {
        p = p.join(fragment);
    }
    return p;
}

/// Compute path under repo's GitDir using a GitRepository, but creates the directory if mk_dir is true
fn repo_dir_gr(
    gr: &GitRepository,
    mk_dir: bool,
    paths: Vec<&str>,
) -> Result<PathBuf, Box<std::error::Error>> {
    repo_dir_path(&gr.gitdir, mk_dir, paths)
}

/// Compute path under repo's GitDir using a raw path as the root, but creates the directory if mk_dir is true
fn repo_dir_path(
    root: &PathBuf,
    mk_dir: bool,
    paths: Vec<&str>,
) -> Result<PathBuf, Box<std::error::Error>> {
    let p = repo_path_path(root, paths);
    if p.exists() {
        if p.is_dir() {
            return Ok(p);
        } else {
            return Err(Box::new(wyagError::WyagError::new(
                "Path already existed as a file. Cannot overwrite file with directory.",
            )));
        }
    }

    if mk_dir {
        let pat: &Path = p.as_path();
        return Ok(p);
    }

    Err(Box::new(std::io::Error::new(
        std::io::ErrorKind::Other, "Failed to create directory. Didn't exist, but was not given the mk_dir option to create subdirectories"
    )))
}

/// Same as repo_path, but create dirname(*path) if absent.  For
/// example, repo_file(r, \"refs\" \"remotes\", \"origin\", \"HEAD\") will create
/// .git/refs/remotes/origin
/// Uses a GitRepository to start
fn repo_file_gr(
    gr: &GitRepository,
    mk_dir: bool,
    paths: Vec<&str>,
) -> Result<PathBuf, Box<std::error::Error>> {
    repo_file_path(&gr.gitdir, mk_dir, paths)
}

/// Same as repo_path, but create dirname(*path) if absent.  For
/// example, repo_file(r, \"refs\" \"remotes\", \"origin\", \"HEAD\") will create
/// .git/refs/remotes/origin
/// Uses a raw path as the root
fn repo_file_path(
    root: &PathBuf,
    mk_dir: bool,
    paths: Vec<&str>,
) -> Result<PathBuf, Box<std::error::Error>> {
    let mut send_down: Vec<&str> = Vec::new();
    if paths.len() > 0 {
        let len_vec = paths.len() - 1;
        send_down = paths[..len_vec].to_vec();
    }

    // checks if the containing dir exists, and if so, returns the full path as handed in.
    // else errors out
    match repo_dir_path(root, mk_dir, send_down) {
        Ok(p) => Ok(repo_path_path(root, paths)),
        Err(m) => Err(m),
    }
}

#[cfg(test)]
mod path_tests {

    use super::*;

    #[test]
    fn repo_path_blank() {
        let gr = GitRepository {
            worktree: "",
            gitdir: PathBuf::new(),
            conf: ini::Ini::new(),
        };

        let p = repo_path_gr(&gr, vec![""]);
        assert_eq!(p.to_string_lossy(), "");
    }

    #[test]
    fn repo_path_pwd() {
        let gr = GitRepository {
            worktree: "",
            gitdir: PathBuf::new().join(""),
            conf: ini::Ini::new(),
        };

        let p = repo_path_gr(&gr, vec!["."]);
        assert_eq!(p.to_string_lossy(), ".");
    }

    #[test]
    fn repo_path_depth_one() {
        let gr = GitRepository {
            worktree: "",
            gitdir: PathBuf::new().join(""),
            conf: ini::Ini::new(),
        };

        let p = repo_path_gr(&gr, vec![".", "this"]);
        assert_eq!(p.to_string_lossy(), ".\\this");
    }

    #[test]
    fn repo_path_depth_two() {
        let gr = GitRepository {
            worktree: "",
            gitdir: PathBuf::new().join(""),
            conf: ini::Ini::new(),
        };

        let p = repo_path_gr(&gr, vec![".", "this", "item.txt"]);
        assert_eq!(p.to_string_lossy(), ".\\this\\item.txt");
    }

    #[test]
    fn repo_path_not_empty() {
        let gr = GitRepository {
            worktree: "",
            gitdir: PathBuf::new().join(""),
            conf: ini::Ini::new(),
        };

        let p = repo_path_gr(&gr, vec![".", "this", "item.txt"]);
        assert_ne!(p.to_string_lossy(), "");
    }

    #[test]
    fn repo_dir_should_return_because_exists_properly() {
        let gr = GitRepository {
            worktree: "",
            gitdir: PathBuf::new().join(""),
            conf: ini::Ini::new(),
        };

        // match repo_dir_gr(&gr, false, vec![".", "this", "item.txt"]) {
        //     Ok(p) =>
        // }
        // assert_ne!(p.to_string_lossy(), "");
    }

    #[test]
    fn repo_dir_should_fail_because_exists_as_file() {
        let gr = GitRepository {
            worktree: "",
            gitdir: PathBuf::new().join(""),
            conf: ini::Ini::new(),
        };

        // match repo_dir_gr(&gr, false, vec![".", "this", "item.txt"]) {
        //     Ok(p) =>
        // }
        // assert_ne!(p.to_string_lossy(), "");
    }

    #[test]
    fn repo_dir_should_return_because_mk_dir_was_on() {
        let gr = GitRepository {
            worktree: "",
            gitdir: PathBuf::new().join(""),
            conf: ini::Ini::new(),
        };

        // match repo_dir_gr(&gr, false, vec![".", "this", "item.txt"]) {
        //     Ok(p) =>
        // }
        // assert_ne!(p.to_string_lossy(), "");
    }

    #[test]
    fn repo_dir_should_fail_because_mk_dir_was_off() {
        let gr = GitRepository {
            worktree: "",
            gitdir: PathBuf::new().join(""),
            conf: ini::Ini::new(),
        };

        // match repo_dir_gr(&gr, false, vec![".", "this", "item.txt"]) {
        //     Ok(p) =>
        // }
        // assert_ne!(p.to_string_lossy(), "");
    }
}

#[cfg(test)]
mod gitrepo_tests {

    use super::*;

    #[test]
    fn ThisRepo() {
        let gr = GitRepository::new(".", false);
        match gr {
            Err(e) => {
                println!("error: {:?}", e);
            }
            Ok(_) => (),
        };
        // let gr = GitRepository {
        //     worktree: "",
        //     gitdir: PathBuf::new(),
        //     conf: ini::Ini::new(),
        // };

        // let p = repo_path_gr(&gr, vec![""]);
        // assert_eq!(p.to_string_lossy(), "");
    }
}
