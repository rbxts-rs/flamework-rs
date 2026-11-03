//! `rbxtsc-fw`: `rbxtsc` with Flamework compiled in (DESIGN §11.6). It takes roblox-ts's `build` arguments and
//! returns its exit codes; Flamework is on for a project whose tsconfig lists `@flamework-experimental/transformer`
//! in `compilerOptions.plugins`, and off otherwise. `--build-info` prints rbxts-rs's lines, then
//! `flamework-rs-version: <this release>`.
#![warn(missing_docs)]

use std::process::ExitCode;

use rbxts_cli::{Cli, Command, parse_args};

#[global_allocator]
static GLOBAL: mimalloc_safe::MiMalloc = mimalloc_safe::MiMalloc;

fn main() -> ExitCode {
    let cli = Cli::new().plugin(flamework_rs::Flamework::new());
    let args: Vec<String> = std::env::args_os().skip(1).map(|a| a.to_string_lossy().into_owned()).collect();
    // The working directory only resolves a build's paths, which `--build-info` does not read.
    if parse_args(args, "") == Ok(Command::BuildInfo) {
        println!("{}flamework-rs-version: {}", cli.build_info(), env!("CARGO_PKG_VERSION"));
        return ExitCode::SUCCESS;
    }
    cli.run()
}
