use clap::Subcommand;

#[derive(Subcommand)]
#[clap(alias = "r")]
#[clap(alias = "revisions")]
pub(crate) enum Commands {
    /// Add a revision
    #[clap(alias = "new")]
    Add {
        /// The storage ID of the Bindle
        app_storage_id: String,
        /// The revision number uploaded to Bindle
        revision_number: String,
    },

    // List all revisions
    List {},
}
