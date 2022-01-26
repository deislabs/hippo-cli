use clap::Subcommand;

#[derive(Subcommand)]
pub(crate) enum Commands {
    Add {
        name: String,

        app_id: String,

        #[clap(short, long)]
        domain: Option<String>,

        #[clap(long)]
        range_rule: Option<String>,

        #[clap(long)]
        revision_id: Option<String>,

        #[clap(long)]
        certificate_id: Option<String>,
    },
    Remove {
        /// The channel ID
        id: String,
    },
}
