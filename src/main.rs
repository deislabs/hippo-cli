mod cli;
mod client;

use cli::Cli;

use clap::Parser;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    cli.execute().await
}
