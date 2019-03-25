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
            eprintln!(
                "{}\n{}",
                "failed to create git repo, directory already existed and was not empty.", err
            );
            process::exit(1)
        }
    } else if config.isCatFile {
        if let Err(err) = lib::cmd_cat_file(config.args[0].as_ref(), config.args[1].as_ref()) {
            eprintln!("Failed to perform cat-file command\n{}", err);
            process::exit(1)
        }
    } else if config.isHashObject {
        let isW: bool = config.args[0]
            .parse()
            .expect("Failed to perform hash-object: somehow the -w flag was misinterpreted as a non-boolean");
        if let Err(err) =
            lib::cmd_hash_object(isW, config.args[1].as_ref(), config.args[2].as_ref())
        {
            eprintln!("Failed to perform hash-object\n{}", err);
            process::exit(1)
        }
    } else if config.isLog {
        if let Err(err) = lib::cmd_log(config.args[0].as_ref()) {
            eprintln!("Failed to perform log: {}", err);
            process::exit(1)
        }
    } else if config.isLsTree {
        if let Err(err) = lib::cmd_ls_tree(config.args[0].as_ref()) {
            eprintln!("Failed to perform ls-tree: {}", err);
            process::exit(1)
        }
    } else if config.isCheckout {
        if let Err(err) = lib::cmd_checkout(config.args[0].as_ref(), config.args[1].as_ref()) {
            eprintln!("Failed to perform checkout: {}", err);
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
    args: Vec<String>,
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

            "cat-file" => {
                c.isCatFile = true;
                let gtype = match args.next() {
                    Some(s) => s.to_owned(),
                    None => {
                        eprintln!("cat-file expects two arguments, received none");
                        process::exit(1)
                    }
                };
                if gtype != "blob" && gtype != "commit" && gtype != "tag" && gtype != "tree" {
                    eprintln!(
                        "first argument to cat-file must be one of [blob, commit, tag, tree]"
                    );
                    process::exit(1)
                }

                let obj = match args.next() {
                    Some(s) => s.to_owned(),
                    None => {
                        eprintln!(
                            "cat-file expects two arguments, but did not receive a second argument"
                        );
                        process::exit(1)
                    }
                };
                c.args = vec![gtype, obj];
                break;
            }

            "hash-object" => {
                let mut path = String::from("x");
                let mut isW = false;
                let mut gitType = String::from("blob");
                c.isHashObject = true;
                while let Some(subarg) = args.next() {
                    match subarg.as_ref() {
                        "-w" => {
                            isW = true;
                        }

                        "-t" => {
                            let gtype = match args.next() {
                                Some(s) => s.to_owned(),
                                None => {
                                    eprintln!("if -t is supplied, a second parameter of [blob, commit, tag, tree] must follow");
                                    process::exit(1)
                                }
                            };
                            if gtype != "blob"
                                && gtype != "commit"
                                && gtype != "tag"
                                && gtype != "tree"
                            {
                                eprintln!(
                                "first argument after -t must be one of [blob, commit, tag, tree]"
                            );
                                process::exit(1)
                            }
                            gitType = gtype;
                        }

                        rest => {
                            path = String::from(rest);
                        }
                    }
                }

                c.args = vec![isW.to_string(), gitType, path];
                break;
            }

            "log" => {
                let commit = match args.next() {
                    Some(s) => s,
                    None => "HEAD",
                };
                c.isLog = true;
                c.args.push(commit.to_owned());
                break;
            }

            "ls-tree" => {
                let sha = match args.next() {
                    Some(s) => s.to_owned(),
                    None => {
                        eprintln!("ls-tree takes a mandatory argument. requires the sha of the item to query.");
                        process::exit(1)
                    }
                };
                c.args.push(sha);
                c.isLsTree = true;
                break;
            }

            "checkout" => {
                c.isCheckout = true;
                let obj = match args.next() {
                    Some(s) => s,
                    None => {
                        eprintln!("checkout requires two arguments, the [commit or tree] to checkout, and the [path to checkout to]. Received neither.");
                        process::exit(1)
                    }
                };
                let path = match args.next() {
                    Some(s) => s,
                    None => {
                        eprintln!("Checkout required two arguments, failed to receive the second.");
                        process::exit(1)
                    }
                };
                c.args.push(obj.to_owned());
                c.args.push(path.to_owned());
                break;
            }

            "add" | "commit" | "merge" | "rebase" | "rev-parse" | "rm" | "show-ref" | "tag" => {
                nyi(arg)
            }

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
