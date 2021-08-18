use clap::App;

mod bindle_pusher;
mod bindle_utils;
mod bindle_writer;
mod command;
mod expander;
mod hippo_notifier;
mod hippofacts;
mod warnings;

/// Indicate which flags are required for bindle builds
#[allow(dead_code)]
enum BindleBuildRequirements {
    /// A Bindle server URL is required
    RequireBindleServer,
    /// An explicit stage directory is required
    RequireStageDirectory,
    /// Both the Bindle server and the stage directory are required
    RequireBindleServerAndStageDirectory,
    /// There are no required arguments for the bindle builds
    NoRequirements,
}

const ABOUT_HIPPO: &str = r#"Create and manage Hippo applications.

The hippo commandline utility provides many tools for managing Hippo applications,
accounts, and configuration. To get started, try 'hippo --help'. To push an existing
Hippo application to the Hippo server, use 'hippo push'.

Many 'hippo' commands operate on a 'HIPPOFACTS' TOML file located in the same directory
in which you are running the 'hippo' command.
"#;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let matches = App::new(env!("CARGO_PKG_NAME"))
        .version(env!("CARGO_PKG_VERSION"))
        .author("Deis Labs")
        .about("The Hippo commandline client")
        .long_about(ABOUT_HIPPO)
        .subcommands(command::apps())
        .get_matches();

    match matches.subcommand() {
        // Make a vague attempt to keep these in alphabetical order
        //Some((push.name(), args)) => println!("push"),
        Some((name, args)) => command::exec(name, args).await,
        _ => Err(anyhow::anyhow!("No matching command. Try 'hippo help'")),
    }
}
