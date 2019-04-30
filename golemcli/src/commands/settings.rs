use structopt::StructOpt;

#[derive(StructOpt, Debug)]
pub enum Section {
    /// Change settings
    Set,
    /// Show current settings
    Show
}