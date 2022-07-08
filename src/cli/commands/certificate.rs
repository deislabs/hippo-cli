use std::path::PathBuf;

use clap::Subcommand;

#[derive(Subcommand)]
#[clap(alias = "cert")]
#[clap(alias = "certs")]
#[clap(alias = "certificates")]
pub(crate) enum Commands {
    /// Add a TLS certificate
    #[clap(alias = "new")]
    Add {
        /// The name of the certificate
        name: String,
        /// The filepath to the public key
        #[clap(parse(from_os_str), value_name = "PUBLIC_KEY")]
        public_key_path: PathBuf,
        /// The filepath to the private key
        #[clap(parse(from_os_str), value_name = "PRIVATE_KEY")]
        private_key_path: PathBuf,
    },

    // List all certificates
    List {},

    /// Remove a TLS certificate
    #[clap(alias = "delete")]
    #[clap(alias = "rm")]
    Remove {
        /// The certificate ID
        id: String,
    },
}
