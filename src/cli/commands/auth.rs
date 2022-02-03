use clap::Subcommand;

#[derive(Subcommand)]
pub(crate) enum Commands {
    /// Create a new Hippo account
    Register {
        /// The Hippo URL
        #[clap(long, default_value = "https://localhost:5309")]
        url: String,
        /// The username
        #[clap(long)]
        username: Option<String>,
        /// The password
        #[clap(long)]
        password: Option<String>,
        /// Should invalid TLS certificates be accepted by the client?
        #[clap(short = 'k', long)]
        danger_accept_invalid_certs: bool,
    },

    /// Log into Hippo
    Login {
        /// The URL to log into Hippo
        #[clap(long, default_value = "https://localhost:5309")]
        url: String,
        /// The username to log into Hippo
        #[clap(long)]
        username: Option<String>,
        /// The password to log into Hippo
        #[clap(long)]
        password: Option<String>,
        /// Should invalid TLS certificates be accepted by the client?
        #[clap(short = 'k', long)]
        danger_accept_invalid_certs: bool,
    },

    /// End the current Hippo login session
    Logout {},

    /// prints the logged in user
    Whoami {},
}
