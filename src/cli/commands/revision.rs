use clap::Subcommand;

#[derive(Subcommand)]
pub(crate) enum Commands {
    Add {
        /// The storage ID of the Bindle
        app_storage_id: String,
        /// The revision number uploaded to Bindle
        revision_number: String,
    },
}
