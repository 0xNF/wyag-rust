extern crate crypto;
extern crate flate2;
extern crate ini;
extern crate linked_hash_map;
use crypto::digest::Digest;
use crypto::sha1;
use flate2::read::ZlibDecoder;
use flate2::write::ZlibEncoder;
use flate2::Compression;
use ini::Ini;
use linked_hash_map::LinkedHashMap;
use std::collections::hash_map::HashMap;
use std::io;
use std::io::Read;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::str;
use std::{error::Error, fmt};

/// GitObject trait
pub trait GitObject {
    /// This function MUST be implemented by an implementation of `GitObject`.
    ///
    /// It must read the object's contents from self.data, a byte string, and do
    /// whatever it takes to convert it into a meaningful representation.  What exactly that means depend on each subclass.
    fn serialize(&self) -> Result<Vec<u8>, WyagError>;
    fn deserialize(&mut self, data: Vec<u8>) -> Result<(), WyagError>;
    fn fmt(&self) -> &[u8];
    fn repo(&self) -> Option<&GitRepository> {
        panic!("Not yet implemented")
    }
}

enum GObj<'a> {
    Tag(GitTag<'a>),
    Commit(GitCommit<'a>),
    Blob(GitBlob<'a>),
    Tree(GitTree<'a>),
}

/// Git Object Concrete Types
struct GitTag<'a> {
    repo: Option<&'a GitRepository<'a>>,
}
struct GitCommit<'a> {
    repo: Option<&'a GitRepository<'a>>,
    kvlm: LinkedHashMap<String, Vec<String>>,
    _data: Vec<u8>,
}

struct GitBlob<'a> {
    repo: Option<&'a GitRepository<'a>>,
    blob_data: Vec<u8>,
}
struct GitTree<'a> {
    repo: Option<&'a GitRepository<'a>>,
    items: Vec<GitTreeLeaf>,
}

impl<'a> GitTag<'a> {
    fn new(repo: Option<&'a GitRepository>, bytes: &[u8]) -> GitTag<'a> {
        GitTag { repo: repo } // TODO NYI
    }
}

impl<'a> GitObject for GitTag<'a> {
    fn serialize(&self) -> Result<Vec<u8>, WyagError> {
        Err(WyagError::new("Serialize on GitTag not yet implenented"))
    }

    fn deserialize(&mut self, data: Vec<u8>) -> Result<(), WyagError> {
        Err(WyagError::new("Deserialize on GitTag not yet implemented"))
    }

    fn fmt(&self) -> &[u8] {
        b"tag"
    }

    // fn repo(&self) -> &GitRepository {
    //     panic!("Not yet implemented");
    // }
}

impl<'a> GitCommit<'a> {
    fn new(repo: Option<&'a GitRepository>, bytes: &[u8]) -> GitCommit<'a> {
        GitCommit {
            repo: repo,
            kvlm: LinkedHashMap::default(),
            _data: bytes.to_vec(),
        } // TODO NYI
    }
}

impl<'a> GitObject for GitCommit<'a> {
    fn serialize(&self) -> Result<Vec<u8>, WyagError> {
        let x = kvlm_serialize(&self.kvlm).into_bytes();
        Ok(x)
    }

    fn deserialize(&mut self, data: Vec<u8>) -> Result<(), WyagError> {
        let mut hm: LinkedHashMap<String, Vec<String>> = LinkedHashMap::new();
        kvlm_parse(data, 0, &mut hm);
        self.kvlm = hm;
        Ok(())
    }

    fn fmt(&self) -> &[u8] {
        b"commit"
    }
}

impl<'a> GitBlob<'a> {
    fn new(repo: Option<&'a GitRepository>, bytes: &[u8]) -> GitBlob<'a> {
        GitBlob {
            blob_data: bytes.to_vec(),
            repo: repo,
        }
    }
}

impl<'a> GitObject for GitBlob<'a> {
    fn serialize(&self) -> Result<Vec<u8>, WyagError> {
        Ok(self.blob_data.to_owned())
    }

    fn deserialize(&mut self, data: Vec<u8>) -> Result<(), WyagError> {
        self.blob_data = data;
        Ok(())
    }

    fn fmt(&self) -> &[u8] {
        b"blob"
    }
}

impl<'a> GitTree<'a> {
    fn new(repo: Option<&'a GitRepository>, bytes: &[u8]) -> GitTree<'a> {
        GitTree {
            repo: repo,
            items: Vec::new(),
        }
    }
}

impl<'a> GitObject for GitTree<'a> {
    fn serialize(&self) -> Result<Vec<u8>, WyagError> {
        tree_serialize(&self)
    }

    fn deserialize(&mut self, data: Vec<u8>) -> Result<(), WyagError> {
        let v = tree_parse(data.as_ref())?;
        self.items = v;
        Ok(())
    }

    fn fmt(&self) -> &[u8] {
        b"tree"
    }
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

// EndRegion: GitRepository

// Region: RepoPaths

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

// EndRegion: RepoPaths

// Region: Reading/Writing Objects

/// Read object object_id from Git repository repo.  Return a
/// GitObject whose exact type depends on the object.
/// 4.3
fn object_read<'a>(repo: &'a GitRepository, sha: &str) -> Result<GObj<'a>, WyagError> {
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

    let mut c: GObj;
    match dfmt {
        b"commit" => c = GObj::Commit(GitCommit::new(Some(repo), &decoded[yIdx + 1..])),
        b"tree" => c = GObj::Tree(GitTree::new(Some(repo), &decoded[yIdx + 1..])),
        b"tag" => c = GObj::Tag(GitTag::new(Some(repo), &decoded[yIdx + 1..])),
        b"blob" => c = GObj::Blob(GitBlob::new(Some(repo), &decoded[yIdx + 1..])),
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

/// Writes the GitObject to its appropriate location in the repo
/// 4.4
fn object_write(obj: &GitObject, actually_write: bool) -> Result<String, WyagError> {
    // serialize the data
    let data = obj.serialize()?;

    // Add header
    let mut result: Vec<u8> = Vec::new();
    result.extend(obj.fmt());
    result.extend(vec![b' ']);
    let us = data.len().to_string().into_bytes();
    result.extend(us);
    result.extend(vec![b'\x00']);
    result.extend(data);

    // compute hash
    let mut sha = crypto::sha1::Sha1::new();
    sha.input(&result);
    let outStr = sha.result_str();

    if actually_write {
        // compute path
        let path = repo_file_gr(
            obj.repo().unwrap(),
            true,
            vec!["objects", &outStr[..2], &outStr[2..]],
        )?;

        let mut e = ZlibEncoder::new(Vec::new(), Compression::default());
        match e.write_all(&result) {
            Ok(_) => (),
            Err(m) => {
                return Err(WyagError::new_with_error(
                    "failed to zlib compress object",
                    Box::new(m),
                ));
            }
        };

        let compressed_bytes = match e.finish() {
            Ok(bytes) => bytes,
            Err(m) => {
                return Err(WyagError::new_with_error(
                    "Failed to finish zlib compressing object",
                    Box::new(m),
                ));
            }
        };

        let compressed_byte_str = "TODO FIXME";
        // TODO get a string from the compressed bytes
        match std::fs::write(path, compressed_byte_str) {
            Ok(_) => (),
            Err(m) => {
                return Err(WyagError::new_with_error(
                    "Failed to write GitObject to file. See inner error for more information.",
                    Box::new(m),
                ));
            }
        };
    }

    Ok(outStr)
}

// TODO not yet implemented
fn object_find<'a>(
    repo: &GitRepository,
    name: &'a str,
    fmt: Option<&str>,
    follow: bool,
) -> Option<&'a str> {
    return Some(name);
}

pub fn cmd_cat_file(gtype: &str, obj: &str) -> Result<(), WyagError> {
    let repo = repo_find(".", false)?;
    cat_file(repo, gtype, obj)
}

fn cat_file<'a>(repo: Option<GitRepository<'_>>, gtype: &str, obj: &str) -> Result<(), WyagError> {
    let repo = match repo {
        Some(gr) => gr,
        None => {
            println!("No git repository was found, cannot cat-file");
            return Ok(());
        }
    };
    let of = match object_find(&repo, obj, Some(gtype), true) {
        Some(s) => s,
        None => {
            println!("no object found for the type: {}", gtype);
            return Ok(());
        }
    };
    let o: Box<dyn GitObject> = match object_read(&repo, of)? {
        GObj::Blob(x) => Box::new(x),
        GObj::Commit(y) => Box::new(y),
        GObj::Tag(z) => Box::new(z),
        GObj::Tree(a) => Box::new(a),
        _ => return Err(WyagError::new("??")),
    };
    let s = (*o).serialize()?.to_vec();
    let st = match String::from_utf8(s) {
        Ok(s) => s,
        Err(m) => {
            return Err(WyagError::new_with_error(
                "Failed to cat file, contained invalid characters",
                Box::new(m),
            ));
        }
    };
    println!("{}", st);
    Ok(())
}

pub fn cmd_hash_object(actually_write: bool, gtype: &str, path: &str) -> Result<(), WyagError> {
    let mut grOpt: Option<GitRepository> = None;
    if actually_write {
        let repo = GitRepository::new(".", false)?;
        grOpt = Some(repo);
    }

    let mut fd = match std::fs::File::open(path) {
        Ok(f) => f,
        Err(m) => {
            return Err(WyagError::new_with_error(
                "Failed to open file at specified path for hash-object",
                Box::new(m),
            ));
        }
    };

    let sha1 = hash_object(&mut fd, gtype, grOpt)?;
    println!("{}", sha1);
    Ok(())
}

fn hash_object<'a>(
    fd: &mut std::fs::File,
    gitType: &str,
    repo: Option<GitRepository<'_>>,
) -> Result<String, WyagError> {
    let mut bytes: Vec<u8> = Vec::new();
    match fd.read_to_end(&mut bytes) {
        Ok(_) => (),
        Err(m) => {
            return Err(WyagError::new_with_error(
                "Failed to perform hash-object",
                Box::new(m),
            ));
        }
    };
    let bytes = bytes.as_slice();

    let mut c: Box<GitObject>;
    match gitType {
        "commit" => c = Box::new(GitCommit::new(repo.as_ref(), bytes)),
        "tree" => c = Box::new(GitTree::new(repo.as_ref(), bytes)),
        "tag" => c = Box::new(GitTag::new(repo.as_ref(), bytes)),
        "blob" => c = Box::new(GitBlob::new(repo.as_ref(), bytes)),
        _ => {
            return Err(WyagError::new(
                format!("Unknown type {}!", gitType).as_ref(),
            ));
        }
    };

    object_write(&*c, true)
}

// EndRegion: Reading/Writing Objects

/// Region: Log

pub fn cmd_log(commit: &str) -> Result<(), WyagError> {
    let repo = match repo_find(".", false)? {
        Some(gr) => gr,
        None => {
            println!("No repository was found, cannot use wyag-log");
            return Ok(());
        }
    };

    println!("digraph wyaglog{{");
    let o = object_find(&repo, commit, None, true);
    if let None = o {
        println!("No such object: {}", commit);
    }
    let mut v: Vec<String> = Vec::new();
    log_graphviz(&repo, String::from(o.unwrap()), &mut v)?;
    println!("}}");
    Ok(())
}

fn log_graphviz<'a>(
    repo: &GitRepository,
    sha: String,
    seen: &mut Vec<String>,
) -> Result<(), WyagError> {
    if seen.contains(&sha) {
        return Ok(());
    }
    let sha2 = sha.clone();
    seen.push(sha);
    let commit: GitCommit = match object_read(repo, sha2.as_ref())? {
        GObj::Commit(y) => y,
        _ => return Err(WyagError::new("??")),
    };

    /* Base Case: the initial commit. */
    let cc = commit.kvlm.clone();
    if !commit.kvlm.contains_key("parent") {
        return Ok(());
    }

    /* Recurse Case */
    let parents = cc["parents"].clone();
    for p in parents {
        println!("c_{} -> c_{}", sha2, &p);
        match log_graphviz(repo, p, seen) {
            Ok(_) => (),
            Err(m) => return Err(m),
        };
    }

    Ok(())
}

fn kvlm_parse(
    raw: Vec<u8>,
    start: usize,
    dict: &mut LinkedHashMap<String, Vec<String>>,
) -> &LinkedHashMap<String, Vec<String>> {
    // Finding the first space
    let space = raw.iter().skip(start).position(|&r| r == b' ');

    // Finding the first newline
    let newline = raw.iter().skip(start).position(|&r| r == b'\n');

    // If a space appears before a newline, we have a new Key value

    // Base Case
    // ====
    // If newline appears first, (or there is no space at all, in which case return -1),
    // we assume a blank line. A blank line means the remainder of the data is the message

    if space.is_none() || newline.unwrap() < space.unwrap() {
        assert_eq!(newline.unwrap(), start);
        let key = "".to_owned();
        let value = match str::from_utf8(&raw[start + 1..]) {
            Ok(s) => s.to_owned(),
            Err(m) => return dict,
        };
        dict.insert(key, vec![value]);
        return dict;
    }

    // Recursive Case
    // ===
    // We read the key-value pair and recurse for the next
    let key = match str::from_utf8(&raw[start..space.unwrap()]) {
        Ok(s) => s.to_owned(),
        Err(m) => {
            panic!("Failed to parse key in kvlm");
            // return Err(WyagError::new_with_error(
            //     "Failed to parse key in kvlm",
            //     Box::new(m),
            // ));
        }
    };

    // Find the end of the value.  Continuation lines begin with a
    // space, so we loop until we find a "\n" not followed by a space.
    let mut end = start;
    loop {
        match raw.iter().skip(end + 1).position(|&r| r == b'\n') {
            Some(i) => end = i,
            None => break,
        }
        if raw[end + 1] != b' ' {
            break;
        }
    }

    // Grab the value
    // Also, drop the leading space on continuation lines
    let rVal = raw[space.unwrap() + 1..end].to_vec();
    let mut value: String = String::from_utf8(rVal).unwrap();
    value = value.replace("\n ", "\n");

    // Don't overwrite values
    if dict.contains_key(&key) {
        let x = dict.get_mut(&key).unwrap();
        x.push(String::from(value));
    }

    kvlm_parse(raw, end + 1, dict)
}

fn kvlm_serialize(hm: &LinkedHashMap<String, Vec<String>>) -> String {
    let mut ret = "".to_owned();
    let mut main = String::new();

    // Output Fields
    for (k, v) in hm.iter() {
        // Skip the message itself
        if k == "" {
            main = String::from(v[0].as_ref());
            continue;
        }
        for val in v {
            ret.push_str(" ");
            ret.push_str(val.replace("\n", "\n ").as_ref());
            ret.push('\n');
        }
    }

    // append message
    ret.push('\n');
    ret.push_str(main.as_ref());

    ret
}

#[cfg(test)]
mod parse_log_tests {
    use super::*;

    #[test]
    fn parse_empty_log() {
        let s = "";
        let mut hm: LinkedHashMap<String, Vec<String>> = LinkedHashMap::new();
        kvlm_parse(s.as_bytes().to_vec(), 0, &mut hm);
        assert_eq!(hm.len(), 0);
    }
}

/// EndRegion: Log

/// Region: Tree

struct GitTreeLeaf {
    mode: Vec<u8>,
    path: Vec<u8>,
    sha: String,
}

fn tree_parse_one(raw: &[u8], start: usize) -> Result<(usize, GitTreeLeaf), WyagError> {
    /* Find the space terminator for the File Mode */
    let x = match raw.iter().skip(start).position(|&r| r == b' ') {
        Some(i) => i,
        None => {
            return Err(WyagError::new(
                "no space found in raw byte stream of tree parse",
            ));
        }
    };
    assert!(x - start == 5 || x - start == 6);

    /* Read the File Mode */
    let mode = raw[start..x].to_vec();

    /* Find the NULL terminator for the path */
    let y = match raw.iter().skip(start).position(|&r| r == b'\x00') {
        Some(i) => i,
        None => {
            return Err(WyagError::new(
                "no null terminator found in raw byte stream of tree parse",
            ));
        }
    };

    /* and read the path */
    let path = raw[x + 1..y].to_vec();

    /* read the SHA1 and convert to a hex string */
    let sha_raw = raw[y + 1..y + 21].to_vec();
    let sha_u32 = sha_parse_u32(&sha_raw);
    let sha_str = sha_parse_str(sha_u32);

    let pos = y + 21;
    let data: GitTreeLeaf = GitTreeLeaf {
        mode: mode,
        path: path,
        sha: sha_str,
    };
    Ok((pos, data))
}

fn tree_parse(raw: &[u8]) -> Result<Vec<GitTreeLeaf>, WyagError> {
    let mut pos: usize = 0;
    let max: usize = raw.len();
    let mut v: Vec<GitTreeLeaf> = Vec::new();

    while pos < max {
        let (pos_m, data) = tree_parse_one(raw, pos)?;
        pos += pos_m;
        v.push(data);
    }

    Ok(v)
}

fn tree_serialize(tree: &GitTree) -> Result<Vec<u8>, WyagError> {
    let mut ret: Vec<u8> = Vec::new();

    for g in &tree.items {
        ret.extend(g.mode.iter());
        ret.push(b' ');
        ret.extend(g.path.iter());
        ret.push(b'\x00');
        let i = u32::from_str_radix(&g.sha, 16);
    }

    Ok(ret)
}

/// TODO TEST ME
fn sha_parse_u32(v: &Vec<u8>) -> u32 {
    let mut buff: [u8; 4] = [0, 0, 0, 0];
    let mut sha: u32 = 0;
    for (i, byte) in v.iter().enumerate() {
        if i % 4 == 0 {
            sha += u32::from_be_bytes(buff);
            buff = [0, 0, 0, 0];
        }
        buff[i % 4] = *byte;
    }
    sha
}

/// TODO TEST ME
fn sha_parse_str(i: u32) -> String {
    format!("{:x}", i)
}

pub fn cmd_ls_tree(name: &str) -> Result<(), WyagError> {
    let repo = match repo_find(".", false)? {
        Some(gr) => gr,
        None => {
            println!("No repository was found, cannot use wyag-log");
            return Ok(());
        }
    };

    let of = match object_find(&repo, name, Some("tree"), true) {
        Some(s) => s,
        None => {
            println!("no object found for the type: {}", "tree");
            return Ok(());
        }
    };
    let tree: GitTree = match object_read(&repo, of)? {
        GObj::Tree(a) => a,
        _ => {
            return Err(WyagError::new(
                "Expected to retrieve a Tree, but received some other type instead",
            ));
        }
    };

    for item in tree.items {
        let mode_a: String = String::from_utf8(item.mode).unwrap();
        let mut first: String = "0".repeat(6);
        first.push_str(mode_a.as_ref());
        /* Git's ls-tree displays the type of the object pointed to. */
        let om = match object_read(&repo, item.sha.as_ref())? {
            GObj::Tree(a) => a.fmt().to_vec(),
            GObj::Tag(t) => t.fmt().to_vec(),
            GObj::Blob(b) => b.fmt().to_vec(),
            GObj::Commit(c) => c.fmt().to_vec(),
            _ => {
                return Err(WyagError::new(
                    "Failed when retrieving object type during ls-tree",
                ));
            }
        };
        let second = match String::from_utf8(om) {
            Ok(s) => s,
            Err(m) => {
                return Err(WyagError::new_with_error(
                    "Failed to parse item type in ls-tree.",
                    Box::new(m),
                ));
            }
        };

        let fourth = match String::from_utf8(item.path) {
            Ok(s) => s,
            Err(m) => {
                return Err(WyagError::new_with_error(
                    "Failed to parse item path in ls-tree.",
                    Box::new(m),
                ));
            }
        };

        println!("{} {} {}\t{}", first, second, item.sha, fourth);
    }

    Ok(())
}

#[cfg(test)]
mod tree_tests {

    #[test]
    fn treeTest() {}
}

/// EndRegion: Tree

/// Region: Checkout

pub fn cmd_checkout(sha: &str, path: &str) -> Result<(), WyagError> {
    let repo = match repo_find(".", false)? {
        Some(gr) => gr,
        None => {
            println!("No repository was found, cannot use wyag-checkout");
            return Ok(());
        }
    };

    let of = match object_find(&repo, sha, None, true) {
        Some(s) => s,
        None => {
            println!("no object found for the type: {}", "commit");
            return Ok(());
        }
    };

    let o: GitTree = match object_read(&repo, of)? {
        // GObj::Blob(x) => Box::new(x),
        GObj::Commit(y) => match object_read(&repo, y.kvlm.get("tree").unwrap()[0].as_ref()) {
            Ok(gobj) => match gobj {
                GObj::Tree(gobj) => gobj,
                _ => {
                    return Err(WyagError::new(
                        "Expected a tree from this commit, but failed to retreive one",
                    ));
                }
            },
            Err(m) => {
                return Err(WyagError::new_with_error(
                    "Expected commit to contain a tree with the value 'tree' but got nothing",
                    Box::new(m),
                ));
            }
        },
        // GObj::Tag(z) => Box::new(z),
        GObj::Tree(a) => a,
        _ => {
            return Err(WyagError::new(
                "encountered an error trying to read object in cmd_checkout. Expected a tree object or a commit object, got something else",
            ));
        }
    };

    /* Verify path is empty directory */
    let p: PathBuf = PathBuf::from(path);
    if p.exists() {
        if !p.is_dir() {
            return Err(WyagError::new("Supplied path was not a directory"));
        } else if let Some(_x) = std::fs::read_dir(&p)
            .expect("can't view this directory. Do you have permission?")
            .next()
        {
            return Err(WyagError::new(
                "Cannot create Git object directory, su pplied path is not empty.",
            ));
        }
    }
    if let Err(m) = std::fs::create_dir(&p) {
        return Err(WyagError::new_with_error(
            "Failed to checkout git object: Error creating directory path",
            Box::new(m),
        ));
    };

    tree_checkout(&repo, o, path)
}

fn tree_checkout(repo: &GitRepository, tree: GitTree, path: &str) -> Result<(), WyagError> {
    for item in tree.items {
        let path_utf8 = match String::from_utf8(item.path) {
            Ok(s) => s,
            Err(m) => {
                return Err(WyagError::new_with_error(
                    "Failed to parse item path tree_checkout.",
                    Box::new(m),
                ));
            }
        };

        let dest: PathBuf = PathBuf::from(path).join(path_utf8);

        match object_read(&repo, &item.sha)? {
            GObj::Tree(a) => {
                if let Err(m) = std::fs::create_dir(&dest) {
                    return Err(WyagError::new_with_error(
                        "Failed to create destination folder during tree_checkout",
                        Box::new(m),
                    ));
                };
                tree_checkout(&repo, a, dest.to_str().unwrap())?;
            }
            GObj::Blob(b) => {
                if let Err(m) = std::fs::write(dest, b.blob_data) {
                    return Err(WyagError::new_with_error(
                        "Failed to write blob data to disk during tree_checkout",
                        Box::new(m),
                    ));
                }
            }
            _ => {
                return Err(WyagError::new(
                    "Expected to retrieve a Tree or a Blob, but received some other type instead",
                ));
            }
        };
    }

    Ok(())
}
/// EndRegion: Checkout

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
        if let Some(e) = &self._err {
            writeln!(f, "Failed to do task: {}", e)
        } else {
            writeln!(f, "Failed to do task")
        }
    }
}

#[cfg(test)]
mod cat_file_tests {

    #[test]
    fn cat_file() {}
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

#[cfg(test)]
mod git_object_read_tests {

    use super::*;

    #[test]
    fn Read_GitCommit_Object_OK() {}

    #[test]
    fn Read_GitCommit_Object_Fail() {}

    #[test]
    fn Read_GitTag_Object_Ok() {}
    #[test]
    fn Read_GitTag_Object_Fail() {}

    #[test]
    fn Read_GitTree_Object_Ok() {}
    #[test]
    fn Read_GitTree_Object_Fail() {}

    #[test]
    fn Read_GitBlob_Object_Ok() {}
    #[test]
    fn Read_GitBlob_Object_Fail() {}
}
