pub(crate) mod app;
pub(crate) mod auth;
pub(crate) mod bindle;
pub(crate) mod certificate;
pub(crate) mod channel;
pub(crate) mod environment_variable;
pub(crate) mod new;
pub(crate) mod revision;

use clap::{AppSettings, Subcommand};
use std::path::PathBuf;

const INVOICE_VERSION_ACCEPTED_VALUES: &[&str] = &["dev", "development", "prod", "production"];

#[derive(Subcommand)]
pub(crate) enum Commands {
    /// Add, update, and remove Applications
    #[clap(subcommand)]
    App(app::Commands),

    /// Register new accounts and log in/out of Hippo
    #[clap(subcommand)]
    Auth(auth::Commands),

    /// Register new accounts and log in/out of Hippo
    #[clap(subcommand)]
    Bindle(bindle::Commands),

    /// Add, update, and remove TLS Certificate
    #[clap(subcommand)]
    Certificate(certificate::Commands),

    /// Add, update, and remove Channels
    #[clap(subcommand)]
    Channel(channel::Commands),

    /// Add, update, and remove environment variables
    #[clap(subcommand)]
    Env(environment_variable::Commands),

    /// Create project-specific files
    #[clap(subcommand)]
    New(new::Commands),

    /// Add and remove revisions
    #[clap(subcommand)]
    Revision(revision::Commands),

    /// Package and upload Hippo artifacts, notifying Hippo
    #[clap(setting(AppSettings::ArgRequiredElseHelp))]
    Push {
        /// The artifacts spec (file or directory containing HIPPOFACTS file)
        #[clap(parse(from_os_str), default_value = ".")]
        path: PathBuf,

        /// How to version the generated invoice
        #[clap(
            short = 'v',
            long,
            default_value = "development",
            possible_values(INVOICE_VERSION_ACCEPTED_VALUES)
        )]
        invoice_version: String,
    },
}
