use clap::Subcommand;

#[derive(Subcommand)]
pub(crate) enum Commands {
    /// Add an application
    Add {
        /// The name of the application
        name: String,
        /// The Bindle ID where releases will be uploaded
        storage_id: String,
    },
    /// Remove an application
    Remove {
        /// The application ID
        id: String,
    },
}
