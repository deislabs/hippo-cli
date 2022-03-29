use std::path::PathBuf;

use clap::Subcommand;

#[derive(Subcommand)]
pub(crate) enum Commands {
    /// Prepare a bindle, but write it to disk instead of sending it over the network
    Prepare {
        /// The artifacts spec (file or directory containing HIPPOFACTS file)
        #[clap(parse(from_os_str), default_value = ".")]
        path: PathBuf,

        /// How to version the generated invoice
        #[clap(
            short = 'v',
            long,
            default_value = "development",
            possible_values(super::INVOICE_VERSION_ACCEPTED_VALUES)
        )]
        invoice_version: String,

        /// Where should the bindle be written to
        #[clap(short, long, parse(from_os_str), default_value = ".hippo")]
        destination: PathBuf,
    },

    /// Package and upload Hippo artifacts without notifying Hippo
    Push {
        /// The artifacts spec (file or directory containing HIPPOFACTS file)
        #[clap(parse(from_os_str), default_value = ".")]
        path: PathBuf,

        /// How to version the generated invoice
        #[clap(
            short = 'v',
            long,
            default_value = "development",
            possible_values(super::INVOICE_VERSION_ACCEPTED_VALUES)
        )]
        invoice_version: String,
    },
    /// Log into Bindle
    Login {
        /// The URL to log into Bindle
        #[clap(long, default_value = "http://localhost:8080/v1")]
        url: String,
        /// The username to log into Bindle
        #[clap(long)]
        username: Option<String>,
        /// The password to log into Bindle
        #[clap(long)]
        password: Option<String>,
        /// Should invalid TLS certificates be accepted by the client?
        #[clap(short = 'k', long)]
        danger_accept_invalid_certs: bool,
    },

    /// End the current Bindle login session
    Logout {},
}
