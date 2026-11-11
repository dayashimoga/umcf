//! `umcf` CLI binary entry point for the Universal Mobile Compute Fabric.

use std::env;
use std::process::ExitCode;
use umcf_cli::run_cli;

fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();
    let (code, output, _is_json) = run_cli(&args);

    if !output.is_empty() {
        println!("{output}");
    }

    code.into_exit_code()
}
