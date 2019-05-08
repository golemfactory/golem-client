use crate::context::CliCtx;
use rustyline::completion::Completer;
use rustyline::config::Configurer;
use rustyline::highlight::Highlighter;
use rustyline::hint::Hinter;
use rustyline::*;
use std::borrow::Cow;
use std::cell::{Ref, RefCell};
use std::iter::Enumerate;
use std::str::Chars;
use structopt::{clap, StructOpt};

#[derive(StructOpt, Debug)]
#[structopt(raw(global_setting = "structopt::clap::AppSettings::ColoredHelp"))]
#[structopt(raw(global_setting = "structopt::clap::AppSettings::VersionlessSubcommands"))]
#[structopt(raw(global_setting = "structopt::clap::AppSettings::NoBinaryName"))]
struct LineArgs {
    /// Return results in JSON format
    #[structopt(long)]
    #[structopt(raw(display_order = "500"))]
    #[structopt(raw(set = "structopt::clap::ArgSettings::Global"))]
    json: bool,

    #[structopt(subcommand)]
    command: Option<super::commands::CommandSection>,
}

struct CliHelper<C: Completer> {
    c: C,
}

mod cmdparse;
mod complete;

impl<C: Completer> Completer for CliHelper<C> {
    type Candidate = C::Candidate;

    fn complete(
        &self,
        line: &str,
        pos: usize,
        ctx: &Context,
    ) -> Result<(usize, Vec<Self::Candidate>)> {
        self.c.complete(line, pos, ctx)
    }
}

impl<C: Completer> Highlighter for CliHelper<C> {}

impl<C: Completer> Hinter for CliHelper<C> {}

impl<C: Completer> Helper for CliHelper<C> {}

pub fn interactive_shell(ctx: &mut CliCtx) {
    let mut editor: Editor<CliHelper<_>> =
        Editor::with_config(Config::builder().auto_add_history(true).build());

    editor.config_mut().auto_add_history();
    editor.set_helper(Some(CliHelper {
        c: complete::completer_for::<LineArgs>(),
    }));

    while let Ok(line) = editor.readline(">> ") {
        match LineArgs::from_iter_safe(line.split_ascii_whitespace()) {
            Ok(LineArgs {
                json: _,
                command: Some(command),
            }) => match command.run_command(ctx) {
                Ok(resp) => ctx.output(resp),
                Err(e) => eprintln!("err: {:?}", e),
            },
            Ok(_) => (),
            Err(clap::Error {
                message,
                kind: _,
                info: _,
            }) => {
                eprintln!("{}", message);
            }
        }
    }
}
