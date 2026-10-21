//! Command line arguments: `openecraft-cli [options] <command> [--<parameter> <value>]…`.
//! The options may stand anywhere; everything else is a parameter of the command.

use openecraft_i18n::tr_fmt;

#[derive(Debug, Default, PartialEq)]
pub struct Options {
    pub project: Option<String>,
    pub server: Option<String>,
    pub token: Option<String>,
    pub lang: Option<String>,
    /// All parameters as one JSON object (`--json '{…}'`).
    pub json: Option<String>,
    pub help: bool,
}

#[derive(Debug, Default, PartialEq)]
pub struct Invocation {
    pub command: Option<String>,
    /// Further words after the command (`help <command>`).
    pub words: Vec<String>,
    pub options: Options,
    /// `--name value` pairs in their order; `None` for a flag without value.
    pub params: Vec<(String, Option<String>)>,
}

pub fn parse(argv: &[String]) -> Result<Invocation, String> {
    let mut inv = Invocation::default();
    let mut i = 0;
    while i < argv.len() {
        let arg = &argv[i];
        i += 1;
        if arg == "-h" || arg == "--help" {
            inv.options.help = true;
            continue;
        }
        let Some(name) = arg.strip_prefix("--").or_else(|| arg.strip_prefix('-').filter(|n| *n == "p")) else {
            if inv.command.is_none() {
                inv.command = Some(arg.clone());
            } else {
                inv.words.push(arg.clone());
            }
            continue;
        };
        let (name, mut value) = match name.split_once('=') {
            Some((n, v)) => (n.to_string(), Some(v.to_string())),
            None => (name.to_string(), None),
        };
        // A value may start with a single dash (device tags like -K1), not with two.
        if value.is_none() && i < argv.len() && !argv[i].starts_with("--") {
            value = Some(argv[i].clone());
            i += 1;
        }
        let option = match name.as_str() {
            "p" | "project" => &mut inv.options.project,
            "server" => &mut inv.options.server,
            "token" => &mut inv.options.token,
            "lang" => &mut inv.options.lang,
            "json" => &mut inv.options.json,
            _ => {
                inv.params.push((name.replace('-', "_"), value));
                continue;
            }
        };
        *option = Some(value.ok_or_else(|| tr_fmt!("Missing value for --{}", name))?);
    }
    Ok(inv)
}

#[cfg(test)]
mod tests;
