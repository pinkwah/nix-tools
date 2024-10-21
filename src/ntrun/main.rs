use std::ffi::OsString;
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{exit, Command, Stdio};
use std::{env, fs, str};

use clap::Parser;
use nix::unistd::{access, AccessFlags};

const COMMON_NAMESPACES: [&str; 4] = [
    "rubyPackages",
    "nodePackages",
    "pythonPackages",
    "haskellPackages",
];

#[derive(Parser, Debug)]
#[command(version, author, about, long_about = None)]
struct Args {
    #[arg(short, long, help = "Verbose mode")]
    verbose: bool,

    #[arg(
        short,
        long,
        help = "Which flake url to use",
        default_value = "nixpkgs"
    )]
    flake: String,

    #[arg(
        short,
        long,
        help = "Don't search for the program in common namespaces"
    )]
    exact: bool,

    #[arg(short, long, help = "Package name. Defaults to first argument")]
    name: Option<String>,

    #[arg(trailing_var_arg = true, required = true)]
    args: Vec<String>,
}

fn nix_build(attr: &str) -> Option<PathBuf> {
    let output = Command::new("nix")
        .args(["build", "--no-link", "--print-out-paths"])
        .arg(&attr)
        .output()
        .ok()?;

    let stdout = str::from_utf8(&output.stdout).ok()?;
    stdout.lines().map(|s| PathBuf::from(s)).next()
}

fn can_nix_eval(attr: &str, verbose: bool) -> bool {
    Command::new("nix")
        .args(["eval", "--raw"])
        .arg(attr)
        .stderr(if verbose {
            Stdio::inherit()
        } else {
            Stdio::null()
        })
        .stdout(if verbose {
            Stdio::inherit()
        } else {
            Stdio::null()
        })
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

fn find_exact_nix_eval(flake: &str, name: &str, verbose: bool) -> Option<String> {
    let check = format!("{}#{}", flake, name);
    if can_nix_eval(check.as_str(), verbose) {
        Some(check)
    } else {
        None
    }
}

fn find_nix_eval(flake: &str, name: &str, verbose: bool) -> Option<String> {
    if let Some(path) = find_exact_nix_eval(flake, name, verbose) {
        return Some(path);
    }

    for namespace in COMMON_NAMESPACES {
        let check = format!("{}#{}.{}", flake, namespace, name);
        if can_nix_eval(check.as_str(), verbose) {
            return Some(check);
        }
    }
    None
}

fn parse_propagated_build_inputs(contents: String) -> Vec<PathBuf> {
    contents
        .split_whitespace()
        .map(|p| Path::new(p).join("bin"))
        .filter(|p| p.is_dir())
        .map(|p| PathBuf::from(p))
        .collect()
}

fn get_binpaths(store: &Path) -> Vec<PathBuf> {
    let mut paths = vec![];
    let path = store.join("bin");
    if path.exists() {
        paths.push(path.to_owned());
    }

    paths.extend(
        fs::read_to_string(store.join("nix-support/propagated-build-inputs"))
            .map(parse_propagated_build_inputs)
            .unwrap_or_default(),
    );

    paths
}

fn find_in_paths(paths: &Vec<PathBuf>, program: &str) -> Option<PathBuf> {
    paths
        .iter()
        .map(|p| p.join(program))
        .filter(|p| access(p, AccessFlags::X_OK).is_ok())
        .next()
}

fn make_path_env(paths: &Vec<PathBuf>) -> OsString {
    let mut pathss: Vec<OsString> = paths.iter().map(|p| p.as_os_str().to_os_string()).collect();
    if let Some(p) = env::var_os("PATH") {
        pathss.push(p);
    }
    pathss.join(&OsString::from(":"))
}

fn main() {
    let args = Args::parse();

    let program = args.args[0].clone();
    let flake = args.flake;

    let expr = if args.exact {
        find_exact_nix_eval(&flake, &program, args.verbose)
    } else {
        find_nix_eval(&flake, &program, args.verbose)
    };

    let expr = expr.unwrap_or_else(|| {
        eprintln!("Could not find {} in {}", &program, flake);
        exit(1);
    });

    let path = nix_build(&expr).unwrap();
    let binpaths = get_binpaths(&path);
    let which = find_in_paths(&binpaths, &program).unwrap();
    let newpath = make_path_env(&binpaths);

    Command::new(which)
        .env("PATH", newpath)
        .args(&args.args[1..])
        .exec();
}
