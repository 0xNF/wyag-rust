extern crate flate2;
extern crate ini;
use flate2::read::ZlibDecoder;
use flate2::write::ZlibEncoder;
use ini::Ini;
use std::io;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::str;
use std::{error::Error, fmt};

/// GitObject trait
pub trait GitObject {
    /// This function MUST be implemented by an implementation of `GitObject`.
    ///
    /// It must read the object's contents from self.data, a byte string, and do
    /// whatever it takes to convert it into a meaningful representation.  What exactly that means depend on each subclass.
    fn serialize(&self) -> Result<(), WyagError>;
    fn deserialize(&self, data: &str) -> Result<Box<GitObject>, WyagError>;
}

/// Git Object Concrete Types
struct GitTag;
struct GitCommit;
struct GitBlob;
struct GitTree;

impl GitTag {
    fn new(repo: &GitRepository, bytes: &[u8]) -> GitTag {
        GitTag // TODO NYI
    }
}

impl GitObject for GitTag {
    fn serialize(&self) -> Result<(), WyagError> {
        Err(WyagError::new("Serialize on GitTag not yet implenented"))
    }

    fn deserialize(&self, data: &str) -> Result<Box<GitObject>, WyagError> {
        Err(WyagError::new("Deserialize on GitTag not yet implemented"))
    }
}

impl GitCommit {
    fn new(repo: &GitRepository, bytes: &[u8]) -> GitCommit {
        GitCommit // TODO NYI
    }
}

impl GitObject for GitCommit {
    fn serialize(&self) -> Result<(), WyagError> {
        Err(WyagError::new("Serialize on GitCommit not yet implenented"))
    }

    fn deserialize(&self, data: &str) -> Result<Box<GitObject>, WyagError> {
        Err(WyagError::new(
            "Deserialize on GitCommit not yet implemented",
        ))
    }
}

impl GitBlob {
    fn new(repo: &GitRepository, bytes: &[u8]) -> GitBlob {
        GitBlob // TODO NYI
    }
}

impl GitObject for GitBlob {
    fn serialize(&self) -> Result<(), WyagError> {
        Err(WyagError::new("Serialize on GitBlob not yet implenented"))
    }

    fn deserialize(&self, data: &str) -> Result<Box<GitObject>, WyagError> {
        Err(WyagError::new("Deserialize on GitBlob not yet implemented"))
    }
}

impl GitTree {
    fn new(repo: &GitRepository, bytes: &[u8]) -> GitTree {
        GitTree // TODO NYI
    }
}

impl GitObject for GitTree {
    fn serialize(&self) -> Result<(), WyagError> {
        Err(WyagError::new("Serialize on GitTree not yet implenented"))
    }

    fn deserialize(&self, data: &str) -> Result<Box<GitObject>, WyagError> {
        Err(WyagError::new("Deserialize on GitTree not yet implemented"))
    }
}

/// Read object object_id from Git repository repo.  Return a
/// GitObject whose exact type depends on the object.
fn object_read(repo: &GitRepository, sha: &str) -> Result<Box<GitObject>, WyagError> {
    // grab the object in question from the filesystem
    let path = repo_file_gr(&repo, false, vec!["objects", &sha[..2], &sha[2..]])?;

    // read the raw bytes of the file.
    let raw = match std::fs::read(path) {
        Ok(bv) => bv,
        Err(m) => {
            return Err(WyagError::new_with_error(
                format!(
                    "Failed to read git object file {}. This error happened before deflating.",
                    sha
                )
                .as_ref(),
                Box::new(m),
            ));
        }
    };

    // decode the zlib enconded data
    let decoded = match decode_reader(raw) {
        Ok(s) => s,
        Err(m) => {
            return Err(WyagError::new_with_error(
                format!("Failed to decode ZLIB encoded byte array: {0}", sha).as_ref(),
                Box::new(m),
            ));
        }
    };

    // read the object type
    let xIdx = match decoded.iter().position(|&r| r == b' ') {
        Some(i) => i,
        None => return Err(WyagError::new(
            format!("Failed decode git object type {}- no space delimeter was found. Is this file corrupted?", sha).as_ref(),
        )),
    };

    // read and validate object size
    let yIdx = match decoded.iter().position(|&r| r == b'\x00') {
        Some(i) => i,
        None => return Err(WyagError::new(
            format!("Failed decode git object type {} - no null delimeter was found. Is this file corrupted?", sha).as_ref(),
        )),
    };

    let size = str::from_utf8(&decoded[xIdx..yIdx]).unwrap(); // todo wyag error here
    let size: usize = size.parse().unwrap(); // todo wyag error here
    if size != decoded.len() - (yIdx - 1) {
        return Err(WyagError::new(
            format!("Malformed object {}, bad length.", sha).as_ref(),
        ));
    }

    let dfmt = &decoded[..xIdx];

    let mut c: Box<GitObject>;
    match dfmt {
        b"commit" => c = Box::new(GitCommit::new(repo, &decoded[yIdx + 1..])),
        b"tree" => c = Box::new(GitTree::new(repo, &decoded[yIdx + 1..])),
        b"tag" => c = Box::new(GitTag::new(repo, &decoded[yIdx + 1..])),
        b"blob" => c = Box::new(GitBlob::new(repo, &decoded[yIdx + 1..])),
        _ => {
            return Err(WyagError::new(
                format!("Unknown type {} for object {}", "", sha).as_ref(), // todo fromat for dfmt
            ));
        }
    };

    Ok(c)
}

fn decode_reader(bytes: Vec<u8>) -> std::io::Result<Vec<u8>> {
    let mut z = ZlibDecoder::new(&bytes[..]);
    let mut byteBuf: Vec<u8> = Vec::new();
    z.read_exact(&mut byteBuf)?;
    Ok(byteBuf)
}

// TODO not yet implemented
fn object_find<'a>(
    repo: &GitRepository,
    name: &'a str,
    fmt: &str,
    follow: bool,
) -> Option<&'a str> {
    return Some(name);
}

/// Git Repository object
pub struct GitRepository<'a> {
    worktree: &'a str,
    gitdir: PathBuf,
    conf: Ini,
}

impl<'a> GitRepository<'a> {
    pub fn new(path: &'a str, force: bool) -> Result<GitRepository, WyagError> {
        // Set up the gitdir
        let git_path = Path::new(path).join(".git");
        if !(force || git_path.is_dir()) {
            let serr = "Not a git path";
            return Err(WyagError::new(serr));
        }

        // Read configuration file in .git/config
        let mut conf = Ini::new();
        match repo_file_path(&git_path, false, vec!["config"]) {
            Ok(p) => {
                // Read if exists
                if p.exists() {
                    match Ini::load_from_file(&p) {
                        Ok(c) => conf = c,
                        Err(m) => {
                            return Err(WyagError::new_with_error(
                                "Failed to read git config file",
                                Box::new(m),
                            ));
                        }
                    };
                } else if !force {
                    return Err(WyagError::new("Configuration file missing"));
                }
            }
            Err(_) => (),
        };

        if !force {
            let core = conf
                .section(Some("core".to_owned()))
                .expect("Expected a section named core, but none existed");
            let repo_format_version = core.get("repositoryformatversion").expect("expected a 'repositoryformatversion' key containing a number under the [core] section, but found nothing");
            let repo_format_version: u32 = repo_format_version.parse().expect("expected 'repositoryformatversion' to contain a valid integer, found an invalid element instead.");
            if repo_format_version != 0 {
                return Err(WyagError::new("Unsupported repo format version"));
            }
        }

        let gr = GitRepository {
            worktree: path,
            gitdir: git_path.to_path_buf(),
            conf: conf,
        };

        Ok(gr)
    }

    /// Creates a new repository at `path`
    pub fn repo_create(path: &str) -> Result<GitRepository, WyagError> {
        let repo = GitRepository::new(path, true)?;

        // check that repo path is either non-existant, or is an empty dir
        let p: PathBuf = PathBuf::from(repo.worktree);

        if p.exists() {
            if p.is_file() {
                return Err(WyagError::new(
                    "Cannot create new repository, supplied path is not a directory.",
                ));
            }
            let mut iter = std::fs::read_dir(p).expect("Failed to read contents of the supplied directory. Do you have permission to view this folder?");
            if let Some(_x) = iter.next() {
                return Err(WyagError::new(
                    "Cannot create new repository, supplied path is not empty.",
                ));
            }
            if let Err(m) = std::fs::create_dir_all(repo.worktree) {
                return Err(WyagError::new_with_error(
                    "failed to create work directory for supplied repository",
                    Box::new(m),
                ));
            }
        }

        if let Err(m) = repo_dir_gr(&repo, true, vec!["branches"]) {
            return Err(WyagError::new(
                "Failed to create directory Branches underneath git main dir",
            ));
        }
        if let Err(m) = repo_dir_gr(&repo, true, vec!["objects"]) {
            return Err(WyagError::new(
                "Failed to create directory objects underneath git main dir",
            ));
        }

        if let Err(m) = repo_dir_gr(&repo, true, vec!["refs", "tags"]) {
            return Err(WyagError::new(
                "Failed to create directory refs/tags underneath git main dir",
            ));
        }

        if let Err(m) = repo_dir_gr(&repo, true, vec!["refs", "heads"]) {
            return Err(WyagError::new(
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
                    return Err(WyagError::new("Failed writing Description file"));
                };
            }
            Err(m) => {
                return Err(WyagError::new(
                    "Failed to create description file under git main",
                ));
            }
        };

        // .git/HEAD
        match repo_file_gr(&repo, false, vec!["HEAD"]) {
            Ok(p) => {
                if let Err(m) = std::fs::write(p, "ref: refs/heads/master\n") {
                    return Err(WyagError::new("Failed writing HEAD file"));
                }
            }
            Err(m) => {
                return Err(WyagError::new("Failed to create HEAD file under git main"));
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
                return Err(WyagError::new(
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

/// Looks for a repository, starting at `path` and recursing back until `/`.
/// To identify something as a repo, checks for the presence of a .git directory.
///
/// # examples
/// repo_find("./", false)  
///
///     Ok => None // if no repo is found, but finding one wasn't required  
///
/// repo_find("./", true)
///
///     Err => ("Failed to find a repository") // if no repo is found, but finding one was required
///
/// repo_find("./" [true/false])
///
///     Ok => Some(gitrepo) // if a repo was found
///
/// repo_find("./", [true/false])
///
///     Err("Failed to read directory") // if some error was encountered
fn repo_find(path: &str, required: bool) -> Result<Option<GitRepository>, WyagError> {
    let p = PathBuf::from(path);
    let real = match p.canonicalize() {
        Ok(p) => p,
        Err(m) => {
            return Err(WyagError::new_with_error(
                "Failed to create canonical path from supplied path",
                Box::new(m),
            ));
        }
    };

    if p.join(".git").is_dir() {
        let gr = GitRepository::new(path, false)?;
        return Ok(Some(gr));
    }

    // # If we haven't returned, recurse in parent
    while let Some(p) = real.parent() {
        if p.join(".git").is_dir() {
            let gr = GitRepository::new(path, false)?;
            return Ok(Some(gr));
        }
    }

    return Ok(None);
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
fn repo_dir_gr(gr: &GitRepository, mk_dir: bool, paths: Vec<&str>) -> Result<PathBuf, WyagError> {
    repo_dir_path(&gr.gitdir, mk_dir, paths)
}

/// Compute path under repo's GitDir using a raw path as the root, but creates the directory if mk_dir is true
fn repo_dir_path(root: &PathBuf, mk_dir: bool, paths: Vec<&str>) -> Result<PathBuf, WyagError> {
    let p = repo_path_path(root, paths);
    if p.exists() {
        if p.is_dir() {
            return Ok(p);
        } else {
            return Err(WyagError::new(
                "Path already existed as a file. Cannot overwrite file with directory.",
            ));
        }
    }

    if mk_dir {
        if let Err(m) = std::fs::create_dir_all(&p) {
            return Err(WyagError::new_with_error(
                "Couldn't create necessary directories",
                Box::new(m),
            ));
        }
        return Ok(p);
    }

    return Err(WyagError::new_with_error("Failed to create directories", Box::new(std::io::Error::new(std::io::ErrorKind::Other, "Failed to create directory. Didn't exist, but was not given the mk_dir option to create subdirectories"))));
}

/// Same as repo_path, but create dirname(*path) if absent.  For
/// example, repo_file(r, \"refs\" \"remotes\", \"origin\", \"HEAD\") will create
/// .git/refs/remotes/origin
/// Uses a GitRepository to start
fn repo_file_gr(gr: &GitRepository, mk_dir: bool, paths: Vec<&str>) -> Result<PathBuf, WyagError> {
    repo_file_path(&gr.gitdir, mk_dir, paths)
}

/// Same as repo_path, but create dirname(*path) if absent.  For
/// example, repo_file(r, \"refs\" \"remotes\", \"origin\", \"HEAD\") will create
/// .git/refs/remotes/origin
/// Uses a raw path as the root
fn repo_file_path(root: &PathBuf, mk_dir: bool, paths: Vec<&str>) -> Result<PathBuf, WyagError> {
    let mut send_down: Vec<&str> = Vec::new();
    if paths.len() > 0 {
        let len_vec = paths.len() - 1;
        send_down = paths[..len_vec].to_vec();
    }

    // checks if the containing dir exists, and if so, returns the full path as handed in.
    // else errors out
    match repo_dir_path(root, mk_dir, send_down) {
        Ok(_p) => Ok(repo_path_path(root, paths)),
        Err(m) => Err(WyagError::new_with_error(
            "Failed to create directory path",
            Box::new(m),
        )),
    }
}

#[derive(Debug, Default)]
pub struct WyagError {
    _message: String,
    _err: Option<Box<dyn Error>>,
}

impl WyagError {
    pub fn new(message: &str) -> WyagError {
        WyagError {
            _message: String::from(message),
            _err: None,
        }
    }

    pub fn new_with_error(message: &str, err: Box<std::error::Error>) -> WyagError {
        WyagError {
            _message: String::from(message),
            _err: Some(err),
        }
    }
}

impl Error for WyagError {
    fn description(&self) -> &str {
        self._message.as_ref()
    }
}

impl fmt::Display for WyagError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "Failed to do task")
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

    fn deleteOldRepo() {
        println!("Deleteing all .\\tt repo");
        let p = PathBuf::from(".\\tt");
        if p.exists() {
            std::fs::remove_dir_all(".\\tt").expect("Failed to delete old git directory");
        }
    }

    #[test]
    fn CreateFromNothing() {
        deleteOldRepo();
        let gr = GitRepository::repo_create(".\\tt");
        match gr {
            Err(e) => {
                println!("error: {:?}", e);
            }
            Ok(_) => {}
        };

        let s = std::fs::read_to_string(".\\tt\\.git\\config");
        assert!(s.unwrap().len() > 0);

        deleteOldRepo();
    }

    #[test]
    fn CreateFromEmptyDirectory() {
        deleteOldRepo();
        std::fs::create_dir(".\\tt");
        let gr = GitRepository::repo_create(".\\tt");
        match gr {
            Err(e) => {
                println!("error: {:?}", e);
            }
            Ok(_) => {}
        };

        let s = std::fs::read_to_string(".\\tt\\.git\\config");
        assert!(s.unwrap().len() > 0);

        deleteOldRepo();
    }

    #[test]
    fn FailToCreateBecauseNonEmpty() {
        deleteOldRepo();

        // create a directory with a file
        std::fs::create_dir(".\\tt").expect("Tried to create test repo directory, but failed");
        std::fs::write(".\\tt\\hello.txt", "sup")
            .expect("Tried to create test repo file, but failed");

        let gr = GitRepository::repo_create(".\\tt");
        assert!(gr.is_err());

        deleteOldRepo();
    }
}
