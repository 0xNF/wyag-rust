use std::path::Path;
mod wyagError;

/// Git Repository object 
pub struct GitRepository<'a> {
    worktree: &'a str,
    gitdir: std::path::PathBuf,
    conf: GitConfig,
}

impl<'a> GitRepository<'a> {
    
    pub fn new(path: &'a str, force: bool)  -> Result<GitRepository, wyagError::WyagError> {

        let gitPath = Path::new(path).join(".git");
        if !force || gitPath.is_dir() {
            let serr = "Not a git path";
            return Err(wyagError::WyagError::new(serr));
        }

        // Read Configuration file
        let gitConfFile = gitPath.join("config");
        let conf = match GitConfig::ReadConfig(&gitConfFile) {
            Ok(c) => c,
            Err(m) => return Err(wyagError::WyagError::new("Failed to read git config file"))
        };

        Ok(
                GitRepository {
                worktree: path,
                gitdir: Path::new(path).join(".git"),
                conf: conf,
            }
        )
    }
}


struct GitConfig {

}

impl GitConfig {

    fn ReadConfig(path: &std::path::PathBuf) -> Result<GitConfig, wyagError::WyagError> {
        Ok(GitConfig {})
    }
}