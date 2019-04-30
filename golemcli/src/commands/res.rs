use structopt::StructOpt;

#[derive(StructOpt, Debug)]
pub enum Section {
    /// Clear provider / requestor resources
    #[structopt(name = "clear")]
    Clear,
    /// Show information on used resources
    #[structopt(name = "show")]
    Show,
}
