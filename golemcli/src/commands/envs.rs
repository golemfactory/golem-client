use structopt::StructOpt;

#[derive(StructOpt, Debug)]
pub enum Section {
    /// Disable environment
    disable,
    /// Enable environment
    enable,
    /// Gets accepted performance multiplier
    perf_mult,
    /// Sets accepted performance multiplier
    perf_mult_set,
    /// Recount performance for an environment
    recount,
    /// Show environments
    show,
}