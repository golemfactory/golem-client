use super::cmdparse::{parse_line, ParseError};
use rustyline::completion::{Candidate, Completer};
use rustyline::{error::ReadlineError, Context};
use structopt::{clap, StructOpt};

struct ClapCompleter<'a, 'b> {
    app: clap::App<'a, 'b>,
}

fn is_alias(app: &clap::App, name: impl AsRef<str>) -> bool {
    &app.p.meta.name == name.as_ref()
}

fn find_matching_sub_commands(app: &clap::App, prefix: &str) -> Vec<String> {
    app.p
        .subcommands
        .iter()
        .filter_map(|cmd| {
            if cmd.p.meta.name.starts_with(prefix) {
                if cmd.p.subcommands.is_empty() {
                    Some(cmd.p.meta.name.clone())
                } else {
                    Some(format!("{} ", cmd.p.meta.name))
                }
            } else {
                None
            }
        }).chain(if "help".starts_with(prefix) { Some("help".to_string())} else { None})
        .collect()
}

impl<'a, 'b> ClapCompleter<'a, 'b> {
    fn for_struct<S: StructOpt>() -> Self {
        let app = S::clap().subcommand(clap::SubCommand::with_name("exit"));
        ClapCompleter { app }
    }

    fn find_completions(&self, cursor_pos: usize, line: &str) -> Option<(usize, Vec<String>)> {
        let mut args = parse_line(line).peekable();
        let mut app = &self.app;

        while args.peek().is_some() {
            let (pos, arg) = match args.next() {
                Some(Ok(v)) => v,
                _e => {
                    return None;
                }
            };
            if pos + arg.len() < cursor_pos {
                match app.p.subcommands.iter().find(|&cmd| is_alias(cmd, &arg)) {
                    Some(s) => {
                        app = &s;
                    }
                    None => {
                        return None;
                    }
                }
            } else {
                return Some((pos, find_matching_sub_commands(app, arg.as_ref())));
            }
        }

        Some((
            cursor_pos,
            app.p
                .subcommands
                .iter()
                .map(|c| c.p.meta.name.to_string())
                .collect(),
        ))
    }
}

impl<'a, 'b> Completer for ClapCompleter<'a, 'b> {
    type Candidate = String;

    fn complete(
        &self,
        line: &str,
        pos: usize,
        _ctx: &Context,
    ) -> Result<(usize, Vec<Self::Candidate>), ReadlineError> {
        match self.find_completions(pos, line) {
            None => Ok((0, Vec::new())),
            Some((p, c)) => Ok((p, c)),
        }
    }
}

pub fn completer_for<S: StructOpt>() -> impl Completer {
    ClapCompleter::for_struct::<S>()
}
