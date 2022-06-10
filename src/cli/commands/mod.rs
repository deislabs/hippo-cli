pub(crate) mod app;
pub(crate) mod certificate;
pub(crate) mod channel;
pub(crate) mod environment_variable;
pub(crate) mod revision;

use clap::Subcommand;

#[derive(Subcommand)]
pub(crate) enum Commands {
    /// Add, update, and remove Applications
    #[clap(subcommand)]
    App(app::Commands),

    /// Add, update, and remove TLS Certificate
    #[clap(subcommand)]
    Certificate(certificate::Commands),

    /// Add, update, and remove Channels
    #[clap(subcommand)]
    Channel(channel::Commands),

    /// Add, update, and remove environment variables
    #[clap(subcommand)]
    Env(environment_variable::Commands),

    /// Log into Hippo
    Login {
        /// The URL to log into Hippo
        #[clap(env = "HIPPO_URL", long, default_value = "http://localhost:5309")]
        url: String,
        /// The username to log into Hippo
        #[clap(env = "HIPPO_USERNAME", long)]
        username: Option<String>,
        /// The password to log into Hippo
        #[clap(env = "HIPPO_PASSWORD", long)]
        password: Option<String>,
        /// Should invalid TLS certificates be accepted by the client?
        #[clap(env, short = 'k', long)]
        danger_accept_invalid_certs: bool,
    },

    /// End the current Hippo login session
    Logout {},

    /// Create a new Hippo account
    Register {
        /// The Hippo URL
        #[clap(env = "HIPPO_URL", long, default_value = "http://localhost:5309")]
        url: String,
        /// The username
        #[clap(env = "HIPPO_USERNAME", long)]
        username: Option<String>,
        /// The password
        #[clap(env = "HIPPO_PASSWORD", long)]
        password: Option<String>,
        /// Should invalid TLS certificates be accepted by the client?
        #[clap(env, short = 'k', long)]
        danger_accept_invalid_certs: bool,
    },

    /// Add and remove revisions
    #[clap(subcommand)]
    Revision(revision::Commands),

    /// prints the logged in user
    Whoami {},
}
