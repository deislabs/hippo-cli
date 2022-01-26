use clap::Subcommand;

#[derive(Subcommand)]
pub(crate) enum Commands {
    /// Create a new Hippo account
    Register {
        /// The Hippo URL
        url: String,
        /// The username
        #[clap(long)]
        username: Option<String>,
        /// The password
        #[clap(long)]
        password: Option<String>,
        /// Should invalid TLS certificates be accepted by the client?
        #[clap(long)]
        danger_accept_invalid_certs: bool,
    },

    /// Log into Hippo
    Login {
        /// The URL to log into Bindle
        bindle_url: String,
        /// The username to log into Bindle
        #[clap(long)]
        bindle_username: Option<String>,
        /// The password to log into Bindle
        #[clap(long)]
        bindle_password: Option<String>,
        /// The URL to log into Hippo
        hippo_url: String,
        /// The username to log into Hippo
        #[clap(long)]
        hippo_username: Option<String>,
        /// The password to log into Hippo
        #[clap(long)]
        hippo_password: Option<String>,
        /// Should invalid TLS certificates be accepted by the client?
        #[clap(long)]
        danger_accept_invalid_certs: bool,
    },

    /// End the current login session
    Logout {},

    /// prints the logged in user
    Whoami {},
}