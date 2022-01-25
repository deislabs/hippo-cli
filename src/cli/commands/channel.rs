use clap::Subcommand;

#[derive(Subcommand)]
pub(crate) enum Commands {
    Add {},
    Remove {
        /// The channel ID
        id: String,
    },
}
