use structopt::StructOpt;

#[derive(StructOpt, Debug)]
pub enum Section {
    /// Clear provider / requestor resources
    clear,
    /// Show information on used resources
    show
}