use std::env;
use std::process;
mod lib;

fn main() {
    let config = match Config::new(env::args()) {
        Err(msg) => {
            eprintln!("{}", msg);
            process::exit(1)
        }
        Ok(c) => c,
    };

    if config.isInit {
        if let Err(err) = lib::GitRepository::repo_create(&config.path) {
            eprintln!("{}", "failed to create git repo");
            process::exit(1)
        }
    }
}

#[derive(Default, Debug)]
/// Config class. Defaults all fields to false.
struct Config {
    isInit: bool,
    isAdd: bool,
    isCatFile: bool,
    isCheckout: bool,
    isCommit: bool,
    isHashObject: bool,
    isLog: bool,
    isLsTree: bool,
    isMerge: bool,
    isRebase: bool,
    isRevParse: bool,
    isRm: bool,
    isShowRef: bool,
    isTag: bool,
    path: String,
}

impl Config {
    fn new(args: env::Args) -> Result<Config, std::io::Error> {
        let mut config = Config {
            ..Default::default()
        };
        parse_args(args.collect(), &mut config);
        println!("{:?}", config);
        Ok(config)
    }
}

fn parse_args(args: Vec<String>, c: &mut Config) {
    if args.len() == 1 {
        print_help_big();
        process::exit(0)
    }

    let mut args = args.iter();
    args.next(); // skip first
    while let Some(arg) = args.next() {
        match arg.as_ref() {
            "-h" => {
                print_help_short();
                process::exit(0)
            }

            "--help" => {
                print_help_big();
                process::exit(0)
            }

            "add" | "cat-file" | "checkout" | "commit" | "hash-object" | "log" | "ls-tree"
            | "merge" | "rebase" | "rev-parse" | "rm" | "show-ref" | "tag" => nyi(arg),

            "init" => {
                c.isInit = true;
                match args.next() {
                    Some(s) => c.path = s.to_string(),
                    None => c.path = ".".to_string(),
                };
                break;
            }
            _ => {
                print_help_short();
                process::exit(0)
            }
        }
    }
}

fn nyi(s: &str) {
    println!("Function {} is not yet implemnented", s);
    process::exit(1)
}

fn print_help_big() {
    print_help_short();
    let s = "
Supported commands are:
    add             adds a file to staging
    cat-file        ?
    checkout        checkouts a file from a commit into the working branch
    commit          adds all staged files to a new HEAD
    hash-object     produces the SHA256 of the specified object
    init            initializes an empty git repository
    log             shows recent commits
    ls-tree         ?
    merge           merges a commit into the working branch
    rebase          collapses commits together
    rev-parse       ?
    rm              removes a file from staging
    show-ref        ?
    tag             ?
";
    println!("{}", s);
}

fn print_help_short() {
    let s = "
usage:  wyat [--version] [--help
        <command> [<args>]
";

    println!("{}", s);
}
