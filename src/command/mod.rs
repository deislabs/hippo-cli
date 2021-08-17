use async_trait::async_trait;
use clap::{App, ArgMatches};

pub(crate) mod newhippo;
pub(crate) mod upload;

/// A command runner is capabile of running particular subcommand.
///
/// It is responsible for defining the command and its args, and then
/// running the command to completion.
#[async_trait]
pub trait CommandRunner {
    /// Create a new Clap app that can be called as a subcommand.
    fn app<'a>() -> clap::App<'a>;
    /// After argument parsing, run the command with the parsed arguments
    async fn run(&self, args: &ArgMatches) -> anyhow::Result<()>;
}

/// Declare which subcommands are exposed to the user.
pub fn apps<'a>() -> Vec<App<'a>> {
    // Subcommands in this list will be registered with the top level Clap app.
    vec![
        upload::Push::app(),
        upload::Bindle::app(),
        upload::Prepare::app(),
        newhippo::NewSubcommand::app(),
    ]
}

/// Execute the apps.
pub async fn exec(name: &str, args: &ArgMatches) -> anyhow::Result<()> {
    match name {
        upload::CMD_BINDLE => {
            let cmd = upload::Bindle {};
            cmd.run(args).await
        }
        upload::CMD_PREPARE => {
            let cmd = upload::Prepare {};
            cmd.run(args).await
        }
        upload::CMD_PUSH => {
            let cmd = upload::Push {};
            cmd.run(args).await
        }
        newhippo::CMD_NEW_HIPPO => {
            let cmd = newhippo::NewHippofacts {};
            cmd.run(args).await
        }
        newhippo::CMD_NEW => {
            let cmd = newhippo::NewSubcommand {};
            cmd.run(args).await
        }
        _ => anyhow::bail!("Unknown command: {}", name),
    }
}
