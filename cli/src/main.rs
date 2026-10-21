//! OpenECraft command line – edits projects and maintains the parts server without a
//! window, and offers the same tools to AI agents as a Model Context Protocol server
//! (`openecraft-cli mcp`).
//!
//! Every command is a tool of [`tools`]; the command line and the MCP server only
//! differ in how they pass the parameters and print the result.

mod args;
mod mcp;
mod parts_server;
mod session;
#[cfg(test)]
mod testing;
mod tools;

use openecraft_i18n::{tr, tr_fmt, Lang};
use session::Session;
use std::process::ExitCode;

fn main() -> ExitCode {
    let argv: Vec<String> = std::env::args().skip(1).collect();
    match run(&argv) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("{}", tr_fmt!("Error: {}", e));
            ExitCode::FAILURE
        }
    }
}

fn run(argv: &[String]) -> Result<(), String> {
    let inv = args::parse(argv)?;
    let lang = inv.options.lang.as_deref();
    if let Some(code) = lang {
        let lang = Lang::from_code(code).ok_or_else(|| tr_fmt!("Unknown language: {}", code))?;
        openecraft_i18n::set_lang(lang);
    }
    let command = inv.command.as_deref().unwrap_or("help");
    if lang.is_none() && command != "mcp" {
        openecraft_i18n::init(None);
    }
    let mut session = Session::new(parts_server::Server::from_options(&inv.options));
    match command {
        "help" => {
            match inv.words.first() {
                Some(name) => print!("{}", tools::help(tools::find(name).ok_or_else(|| unknown(name))?)),
                None => print!("{}", usage()),
            }
            Ok(())
        }
        "mcp" => mcp::serve(&mut session, std::io::stdin().lock(), std::io::stdout().lock()),
        "version" => {
            println!("openecraft-cli {}", env!("CARGO_PKG_VERSION"));
            Ok(())
        }
        name => {
            let tool = tools::find(name).ok_or_else(|| unknown(name))?;
            if let Some(word) = inv.words.first() {
                return Err(tr_fmt!("Unexpected argument: {}", word));
            }
            if inv.options.help {
                print!("{}", tools::help(tool));
                return Ok(());
            }
            let params = tools::from_cli(tool, inv.options.json.as_deref(), &inv.params)?;
            if tool.needs_project {
                let path = inv.options.project.as_deref().ok_or_else(|| tr("This command needs a project: --project <file>").to_string())?;
                session.open(path)?;
            }
            let result = tools::call(&mut session, tool, &params)?;
            // The command line has no session: changes are written back at once.
            if session.unsaved() && session.path.is_some() {
                session.save(None)?;
            }
            match result {
                serde_json::Value::String(text) => println!("{text}"),
                value => println!("{}", serde_json::to_string_pretty(&value).unwrap_or_default()),
            }
            Ok(())
        }
    }
}

fn unknown(name: &str) -> String {
    tr_fmt!("Unknown command: {}. Run openecraft-cli help for a list.", name)
}

fn usage() -> String {
    let mut out = tr(
        "Usage: openecraft-cli [--project <file>] [--server <url>] [--token <token>] [--lang <code>] <command> [--<parameter> <value>]…\n\nopenecraft-cli mcp        runs a Model Context Protocol server on stdin / stdout\nopenecraft-cli <command> --help   shows the parameters of a command\n\nCommands:\n",
    )
    .to_string();
    for tool in tools::all() {
        out.push_str(&format!("  {:<18} {}\n", tool.name, tr(tool.help)));
    }
    out
}
