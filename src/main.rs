mod bindle;
mod cli;
mod expander;
mod hippo;
mod hippofacts;
mod warnings;

use cli::Cli;

use clap::Parser;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    cli.execute().await
}
