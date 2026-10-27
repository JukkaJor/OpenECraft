//! OpenECraft command line – edits projects and maintains the parts server without a
//! window, and offers the same tools to AI agents as a Model Context Protocol server
//! (`openecraft-cli mcp`).
//!
//! Every command is a tool of [`tools`]; the command line ([`command_line`]) and the
//! MCP server only differ in how they pass the parameters and print the result.

mod args;
mod command_line;
mod mcp;
mod parts_server;
mod session;
#[cfg(test)]
mod testing;
mod tools;
mod vault;

use openecraft_i18n::tr_fmt;
use std::process::ExitCode;

fn main() -> ExitCode {
    let argv: Vec<String> = std::env::args().skip(1).collect();
    match command_line::run(&argv) {
        Ok(out) => {
            print!("{out}");
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("{}", tr_fmt!("Error: {}", e));
            ExitCode::FAILURE
        }
    }
}
