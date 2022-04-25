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

    // List all environment variables
    List { },

    /// Remove an environment variable
    #[clap(alias ="delete")]
    #[clap(alias ="rm")]
    Remove {
        /// The environment variable ID
        id: String,
    },
}
