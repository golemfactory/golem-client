use structopt::StructOpt;

#[derive(StructOpt, Debug)]
pub enum Section {
    /// Disable environment
    #[structopt(name = "disable")]
    Disable,
    /// Enable environment
    #[structopt(name = "enable")]
    Enable,

    /// Gets accepted performance multiplier
    #[structopt(name = "perf_mult")]
    PerfMult,

    /// Sets accepted performance multiplier
    #[structopt(name = "perf_mult_set")]
    PerfMultSet,

    /// Recount performance for an environment
    #[structopt(name = "recount")]
    Recount,
    /// Show environments
    #[structopt(name = "show")]
    Show,
}
