use clap::Subcommand;

#[derive(Subcommand)]
#[clap(alias ="e")]
#[clap(alias ="envvar")]
#[clap(alias ="envvars")]
#[clap(alias ="environmentvariable")]
#[clap(alias ="environmentvariables")]
pub(crate) enum Commands {
    /// Add an environment variable
    #[clap(alias ="new")]
    Add {
        /// The environment variable key
        key: String,
        /// The environment variable value
        value: String,
        /// The channel ID this environment variable will be bound to
        channel_id: String,
    },

    // List all environment variables bound to a channel
    List {
        /// The channel ID we want to lookup
        channel_id: String,
    },

    /// Remove an environment variable
    #[clap(alias ="delete")]
    #[clap(alias ="rm")]
    Remove {
        /// The channel ID we want to remove this environment variable from
        channel_id: String,
        /// The environment variable ID
        id: String,
    },
}
