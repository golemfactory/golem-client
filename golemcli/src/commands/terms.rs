use structopt::StructOpt;

#[derive(StructOpt, Debug)]
pub enum Section {
    /// Accept terms of use
    Accept,
    /// Show terms of use
    Show
}

