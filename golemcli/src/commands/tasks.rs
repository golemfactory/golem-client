use structopt::StructOpt;

#[derive(StructOpt, Debug)]
pub enum Section {
    Abort,
    /// Create a task from file. Note: no client-side validation is performed yet.
    /// This will change in the future
    Create,
    /// Delete a task
    Delete,
    /// Dump an existing task
    Dump,
    /// Deletes all tasks
    Purge,
    /// Restart a task
    Restart,
    /// Restart given subtasks from a task
    RestartSubtasks,
    /// Show task details
    Show,
    /// Show statistics for tasks
    Stats,
    /// Show sub-tasks
    Subtasks,
    /// Dump a task template
    Template,
    /// Show statistics for unsupported tasks
    Unsupport,
}
