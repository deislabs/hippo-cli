use clap::Subcommand;

#[derive(Subcommand)]
#[clap(alias = "c")]
#[clap(alias = "channels")]
pub(crate) enum Commands {
    /// Add a channel
    #[clap(alias = "new")]
    Add {
        /// The name of the channel
        name: String,

        /// The application ID this channel is bound to
        app_id: String,

        /// The domain name used to serve requests for this channel
        #[clap(short, long)]
        domain: Option<String>,

        /// if specified, informs hippo to deploy a revision that matches this rule
        #[clap(long)]
        range_rule: Option<String>,

        /// if specified, informs hippo to deploy this revision and ONLY this revision
        #[clap(long)]
        revision_id: Option<String>,

        /// the TLS certificate that should be bound to this channel
        #[clap(long)]
        certificate_id: Option<String>,
    },

    // List all channels
    List {},

    /// Remove a channel
    #[clap(alias = "delete")]
    #[clap(alias = "rm")]
    Remove {
        /// The channel ID
        id: String,
    },

    /// Fetch logs
    Logs {
        /// The channel ID
        id: String,
    }
}
