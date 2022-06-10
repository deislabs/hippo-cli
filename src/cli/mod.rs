mod commands;

use commands::{
    app::Commands as AppCommands, certificate::Commands as CertificateCommands,
    channel::Commands as ChannelCommands, environment_variable::Commands as EnvCommands,
    revision::Commands as RevisionCommands, Commands,
};

use crate::client::{Client, ConnectionInfo};

use clap::Parser;
use dialoguer::{Input, Password};
use dirs::config_dir;
use hippo_openapi::models::{ChannelRevisionSelectionStrategy, TokenInfo};
use log::LevelFilter;
use serde::{Deserialize, Serialize};
use std::{
    fs::{self, File},
    io::BufReader,
    path::PathBuf,
};

const ABOUT_HIPPO: &str = r#"Create and manage Hippo applications.

The hippo commandline utility provides many tools for managing Hippo applications,
accounts, and configuration. To get started, try 'hippo --help'. To push an existing
Hippo application to the Hippo server, use 'hippo push'.

Many 'hippo' commands operate on a 'HIPPOFACTS' TOML file located in the same directory
in which you are running the 'hippo' command.
"#;

#[derive(Serialize, Deserialize)]
struct HippoConfig {
    danger_accept_invalid_certs: bool,
    token_info: Option<TokenInfo>,
    username: String,
    url: String,
}

impl Default for HippoConfig {
    fn default() -> Self {
        Self {
            danger_accept_invalid_certs: false,
            token_info: None,
            username: "".to_owned(),
            url: "http://localhost:5309".to_owned(),
        }
    }
}

/// The Hippo commandline client
#[derive(Parser)]
#[clap(name = "hippo")]
#[clap(author, version, about, long_about = ABOUT_HIPPO)]
pub struct Cli {
    /// Sets a custom config file
    #[clap(short, long, parse(from_os_str), value_name = "FILE")]
    config: Option<PathBuf>,

    /// Turn debugging information on
    #[clap(short, long, parse(from_occurrences))]
    verbose: usize,

    #[clap(subcommand)]
    command: commands::Commands,
}

impl Cli {
    pub async fn execute(&self) -> anyhow::Result<()> {
        let hippo_config_path = match &self.config {
            Some(p) => p.clone(),
            None => PathBuf::from(
                config_dir()
                    .map(|h| h.join("hippo").join("hippo.json"))
                    .unwrap(),
            ),
        };

        // TODO: switch from std::fs to tokio::fs once serde_json implements tokio support
        // https://github.com/serde-rs/json/issues/316
        let mut hippo_conf: HippoConfig = Default::default();
        if hippo_config_path.exists() {
            let file = File::open(hippo_config_path.clone())?;
            let reader = BufReader::new(file);
            hippo_conf = serde_json::from_reader(reader)?;
        }

        let mut builder = env_logger::builder();
        builder.parse_default_env();
        builder.filter_level(match self.verbose {
            0 => LevelFilter::Off,
            1 => LevelFilter::Error,
            2 => LevelFilter::Warn,
            3 => LevelFilter::Info,
            4 => LevelFilter::Debug,
            _ => LevelFilter::Trace,
        });

        builder.init();

        let hippo_client = Client::new(ConnectionInfo {
            url: hippo_conf.url,
            danger_accept_invalid_certs: hippo_conf.danger_accept_invalid_certs,
            api_key: hippo_conf.token_info.map_or(None, |t| t.token),
        });

        match &self.command {
            Commands::App(AppCommands::Add { name, storage_id }) => {
                let id = hippo_client
                    .add_app(name.to_owned(), storage_id.to_owned())
                    .await?;
                println!("Added App {} (ID = '{}')", name, id);
                println!("IMPORTANT: save this App ID for later - you will need it to update and/or delete the App");
            }

            Commands::App(AppCommands::List { }) => {
                let apps = hippo_client.list_apps().await?;
                println!("{}", serde_json::to_string_pretty(&apps.apps)?);
            }

            Commands::App(AppCommands::Remove { id }) => {
                hippo_client.remove_app(id.to_owned()).await?;
                println!("Removed App {}", id);
            }

            Commands::Certificate(CertificateCommands::Add {
                name,
                public_key_path,
                private_key_path,
            }) => {
                // open files and read their contents
                let public_key = fs::read_to_string(public_key_path)?;
                let private_key = fs::read_to_string(private_key_path)?;
                let id = hippo_client
                    .add_certificate(name.to_owned(), public_key, private_key)
                    .await?;
                println!("Added Certificate {} (ID = '{}')", name, id);
                println!("IMPORTANT: save this Certificate ID for later - you will need it to update and/or delete the Certificate");
            }

            Commands::Certificate(CertificateCommands::List { }) => {
                let certificates = hippo_client.list_certificates().await?;
                println!("{}", serde_json::to_string_pretty(&certificates.certificates)?);
            }

            Commands::Certificate(CertificateCommands::Remove { id }) => {
                hippo_client.remove_certificate(id.to_owned()).await?;
                println!("Removed Certificate {}", id);
            }

            Commands::Channel(ChannelCommands::Add {
                app_id,
                name,
                domain,
                range_rule,
                revision_id,
                certificate_id,
            }) => {
                if range_rule.is_some() && revision_id.is_some() {
                    anyhow::bail!("cannot specify both a range rule and a revision ID");
                }
                let revision_selection_strategy = match (range_rule, revision_id) {
                    (Some(_), None) => ChannelRevisionSelectionStrategy::UseRangeRule,
                    (None, Some(_)) => ChannelRevisionSelectionStrategy::UseSpecifiedRevision,
                    _ => ChannelRevisionSelectionStrategy::UseRangeRule,
                };
                let id = hippo_client
                    .add_channel(
                        app_id.to_owned(),
                        name.to_owned(),
                        domain.to_owned(),
                        revision_selection_strategy,
                        range_rule.to_owned(),
                        revision_id.to_owned(),
                        certificate_id.to_owned(),
                    )
                    .await?;
                println!("Added Channel {} (ID = '{}')", name, id);
                println!("IMPORTANT: save this Channel ID for later - you will need it to update and/or delete the Channel");
            }

            Commands::Channel(ChannelCommands::List { }) => {
                let channels = hippo_client.list_channels().await?;
                println!("{}", serde_json::to_string_pretty(&channels.channels)?);
            }

            Commands::Channel(ChannelCommands::Remove { id }) => {
                hippo_client.remove_channel(id.to_owned()).await?;
                println!("Removed Channel {}", id);
            }

            Commands::Env(EnvCommands::Add {
                key,
                value,
                channel_id,
            }) => {
                let id = hippo_client
                    .add_environment_variable(
                        key.to_owned(),
                        value.to_owned(),
                        channel_id.to_owned(),
                    )
                    .await?;
                println!("Added Environment Variable {} (ID = '{}')", key, id);
                println!("IMPORTANT: save this Environment Variable ID for later - you will need it to update and/or delete the Environment Variable");
            }

            Commands::Env(EnvCommands::List { }) => {
                let envs = hippo_client.list_environmentvariables().await?;
                println!("{}", serde_json::to_string_pretty(&envs.environment_variables)?);
            }

            Commands::Env(EnvCommands::Remove { id }) => {
                hippo_client
                    .remove_environment_variable(id.to_owned())
                    .await?;
                println!("Removed Environment Variable {}", id);
            }

            Commands::Login {
                url,
                username,
                password,
                danger_accept_invalid_certs,
            } => {
                let h_username: String = match username {
                    Some(u) => u.to_owned(),
                    None => Input::new().with_prompt("Enter username").interact_text()?,
                };
                let h_password: String = match password {
                    Some(p) => p.to_owned(),
                    None => Password::new().with_prompt("Enter password").interact()?,
                };
                let hippo_client = Client::new(ConnectionInfo {
                    url: url.to_owned(),
                    danger_accept_invalid_certs: *danger_accept_invalid_certs,
                    api_key: None,
                });
                let token = hippo_client.login(h_username.clone(), h_password).await?;
                hippo_conf.danger_accept_invalid_certs = *danger_accept_invalid_certs;
                hippo_conf.username = h_username;
                hippo_conf.url = url.to_owned();
                hippo_conf.token_info = Some(token);
                if !hippo_config_path.exists() && hippo_config_path.ancestors().count() != 0 {
                    fs::create_dir_all(hippo_config_path.parent().unwrap())?;
                }
                serde_json::to_writer(File::create(hippo_config_path)?, &hippo_conf)?;
                println!("Logged in as {}", hippo_conf.username);
            }

            Commands::Logout {} => {
                hippo_conf = Default::default();
                if !hippo_config_path.exists() && hippo_config_path.ancestors().count() != 0 {
                    fs::create_dir_all(hippo_config_path.parent().unwrap())?;
                }
                serde_json::to_writer(File::create(hippo_config_path)?, &hippo_conf)?;
                println!("Logged out");
            }

            Commands::Register {
                url,
                username,
                password,
                danger_accept_invalid_certs,
            } => {
                let uname: String = match username {
                    Some(u) => u.to_owned(),
                    None => Input::new().with_prompt("Enter username").interact_text()?,
                };
                let pword: String = match password {
                    Some(p) => p.to_owned(),
                    None => Password::new()
                        .with_prompt("Enter password")
                        .with_confirmation("Confirm password", "Passwords do not match")
                        .interact()?,
                };
                let hippo_client = Client::new(ConnectionInfo {
                    url: url.to_owned(),
                    danger_accept_invalid_certs: *danger_accept_invalid_certs,
                    api_key: None,
                });
                hippo_client.register(uname.clone(), pword).await?;
                println!("Registered {}", uname);
            }

            Commands::Revision(RevisionCommands::Add {
                app_storage_id,
                revision_number,
            }) => {
                hippo_client
                    .add_revision(app_storage_id.to_owned(), revision_number.to_owned())
                    .await?;
                println!("Added Revision {}", revision_number);
            }

            Commands::Revision(RevisionCommands::List {}) => {
                let revisions = hippo_client.list_revisions().await?;
                println!("{}", serde_json::to_string_pretty(&revisions.revisions)?);
            }

            Commands::Whoami {} => {
                println!("{}", hippo_conf.username);
            }
        }

        Ok(())
    }
}
