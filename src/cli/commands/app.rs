use clap::Subcommand;

#[derive(Subcommand)]
#[clap(alias = "a")]
#[clap(alias = "apps")]
pub(crate) enum Commands {
    /// Add an application
    #[clap(alias = "new")]
    Add {
        /// The name of the application
        name: String,
        /// The Bindle ID where releases will be uploaded
        storage_id: String,
    },

    /// List all apps
    List {},

    /// Remove an application
    #[clap(alias = "delete")]
    #[clap(alias = "rm")]
    Remove {
        /// The application ID
        id: String,
    },
}
