use std::path::Path;

macro_rules! repo_path {
    ($( $x:expr ), *) => {
        let mut p = Path::new("");
        &(
            p = p.join($x);
        )*
        p
    };
}