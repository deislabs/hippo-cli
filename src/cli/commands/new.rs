use std::path::PathBuf;

use clap::Subcommand;

#[derive(Subcommand)]
pub(crate) enum Commands {
    /// Creates a new hippo project with a HIPPOFACTS file
    Hippofacts {
        // The name of the new application
        name: String,

        /// The directory or file into which the HIPPOFACTS file should be written.
        ///
        /// If this is a directory, a HIPPOFACTS file will be written to the directory. Otherwise, a new file will be created with the given name.
        #[clap(
            short,
            long,
            parse(from_os_str),
            value_name = "FILE_OR_DIR",
            default_value = "."
        )]
        destination: PathBuf,

        /// The name(s) and email(s) of the author(s): 'First Last <user@example.com>'
        #[clap(short, long)]
        authors: Vec<String>,

        /// The path to the Wasm module. Example: 'bin/main.wasm'
        #[clap(
            short,
            long,
            parse(from_os_str),
            value_name = "MODULE.WASM",
            default_value = "main.wasm"
        )]
        module: PathBuf,
    },
}
