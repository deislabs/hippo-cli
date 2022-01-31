use clap::Subcommand;

#[derive(Subcommand)]
pub(crate) enum Commands {
    /// Add a revision
    Add {
        /// The storage ID of the Bindle
        app_storage_id: String,
        /// The revision number uploaded to Bindle
        revision_number: String,
    },
}
