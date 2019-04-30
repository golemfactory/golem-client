use structopt::StructOpt;

#[derive(StructOpt, Debug)]
pub enum Section {
    /// Restart a subtask
    Restart,
    /// Show subtask details
    Show
}