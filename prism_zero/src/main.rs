//! `prism_zero run <files...> [-- args...]`, `prism_zero test <files...> [scripts...]`,
//! `prism_zero check <files...>`: see `driver`.

use std::process::ExitCode;

use prism_zero::driver::{check_files, read_files, run_files, test_files};

const USAGE: &str = "usage:
  prism_zero run <files.prism...> [-- args...]      run @main under the CLI World
  prism_zero test <files.prism...> [scripts.prismtest...]
                                                    run test scripts under the Test World
  prism_zero check <files.prism...>                 parse and resolve only";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let Some(cmd) = args.first() else {
        eprintln!("{USAGE}");
        return ExitCode::from(2);
    };
    let rest = &args[1..];
    let (paths, program_args): (Vec<String>, Vec<String>) = match rest.iter().position(|a| a == "--") {
        Some(i) => (rest[..i].to_vec(), rest[i + 1..].to_vec()),
        None => (rest.to_vec(), vec![]),
    };
    let (sources, scripts): (Vec<String>, Vec<String>) = paths.into_iter().partition(|p| !p.ends_with(".prismtest"));
    let files = match read_files(&sources) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("{e}");
            return ExitCode::from(2);
        }
    };
    let out = match cmd.as_str() {
        "run" => run_files(&files, program_args),
        "test" => match read_files(&scripts) {
            Ok(s) => test_files(&files, &s),
            Err(e) => {
                eprintln!("{e}");
                return ExitCode::from(2);
            }
        },
        "check" => check_files(&files),
        _ => {
            eprintln!("{USAGE}");
            return ExitCode::from(2);
        }
    };
    print!("{}", out.stdout);
    eprint!("{}", out.stderr);
    ExitCode::from(out.code)
}
