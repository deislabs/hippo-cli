use std::path::PathBuf;

use clap::Subcommand;

#[derive(Subcommand)]
pub(crate) enum Commands {
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
    Remove {
        /// The certificate ID
        id: String,
    },
}
