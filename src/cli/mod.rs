mod commands;

use commands::{
    app::Commands as AppCommands, auth::Commands as AuthCommands,
    certificate::Commands as CertificateCommands, channel::Commands as ChannelCommands,
    environment_variable::Commands as EnvCommands, new::Commands as NewCommands,
    revision::Commands as RevisionCommands, Commands,
};
use itertools::Itertools;

use crate::{
    bindle::{client::ConnectionInfo as BindleConnectionInfo, pusher::push_all, writer::Writer},
    expander::{ExpansionContext, InvoiceVersioning},
    hippo::{Client, ConnectionInfo},
    hippofacts::{BindleSpec, HippoFacts, HippoFactsEntry, RawHandler, RawHippoFacts},
};

use clap::Parser;
use dialoguer::{Input, Password};
use dirs::config_dir;
use hippo_openapi::models::TokenInfo;
use log::{warn, LevelFilter};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    env::{current_dir, temp_dir},
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
struct Config {
    bindle_url: String,
    bindle_username: Option<String>,
    bindle_password: Option<String>,
    danger_accept_invalid_certs: bool,
    hippo_token_info: Option<TokenInfo>,
    hippo_username: String,
    hippo_url: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            bindle_url: "http://localhost:8080/v1".to_owned(),
            bindle_username: None,
            bindle_password: None,
            danger_accept_invalid_certs: false,
            hippo_token_info: None,
            hippo_username: "".to_owned(),
            hippo_url: "https://localhost:5309".to_owned(),
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
        let config_path = match &self.config {
            Some(p) => p.clone(),
            None => PathBuf::from(
                config_dir()
                    .map(|h| h.join("hippo").join("config.json"))
                    .unwrap(),
            ),
        };

        // TODO: switch from std::fs to tokio::fs once serde_json implements tokio support
        // https://github.com/serde-rs/json/issues/316
        let mut conf: Config = Default::default();
        if config_path.exists() {
            let file = File::open(config_path.clone())?;
            let reader = BufReader::new(file);
            conf = serde_json::from_reader(reader)?;
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
            url: conf.hippo_url,
            danger_accept_invalid_certs: conf.danger_accept_invalid_certs,
            api_key: conf.hippo_token_info.map_or(None, |t| t.token),
        });

        let bindle_connection_info = BindleConnectionInfo::new(
            conf.bindle_url,
            conf.danger_accept_invalid_certs,
            conf.bindle_username,
            conf.bindle_password,
        );

        match &self.command {
            Commands::App(AppCommands::Add { name, storage_id }) => {
                let id = hippo_client
                    .add_app(name.to_owned(), storage_id.to_owned())
                    .await?;
                println!("Added App {} (ID = '{}')", name, id);
                println!("IMPORTANT: save this App ID for later - you will need it to update and/or delete the App (for now)");
            }

            Commands::App(AppCommands::Remove { id }) => {
                hippo_client.remove_app(id.to_owned()).await?;
                println!("Removed App {}", id);
            }

            Commands::Auth(AuthCommands::Register {
                url,
                username,
                password,
                danger_accept_invalid_certs,
            }) => {
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

            Commands::Auth(AuthCommands::Login {
                bindle_url,
                bindle_username,
                bindle_password,
                hippo_url,
                hippo_username,
                hippo_password,
                danger_accept_invalid_certs,
            }) => {
                let h_username: String = match hippo_username {
                    Some(u) => u.to_owned(),
                    None => Input::new()
                        .with_prompt("Enter Hippo username")
                        .interact_text()?,
                };
                let h_password: String = match hippo_password {
                    Some(p) => p.to_owned(),
                    None => Password::new()
                        .with_prompt("Enter Hippo password")
                        .interact()?,
                };
                let hippo_client = Client::new(ConnectionInfo {
                    url: hippo_url.to_owned(),
                    danger_accept_invalid_certs: *danger_accept_invalid_certs,
                    api_key: None,
                });
                let token = hippo_client.login(h_username.clone(), h_password).await?;
                conf.danger_accept_invalid_certs = *danger_accept_invalid_certs;
                conf.hippo_username = h_username;
                conf.hippo_url = hippo_url.to_owned();
                conf.hippo_token_info = Some(token);
                conf.bindle_url = bindle_url.to_owned();
                conf.bindle_username = bindle_username.to_owned();
                conf.bindle_password = bindle_password.to_owned();
                if !config_path.exists() && config_path.ancestors().count() != 0 {
                    fs::create_dir_all(config_path.parent().unwrap())?;
                }
                serde_json::to_writer(File::create(config_path)?, &conf)?;
                println!("Logged in as {}", conf.hippo_username);
            }

            Commands::Auth(AuthCommands::Logout {}) => {
                conf = Default::default();
                if !config_path.exists() && config_path.ancestors().count() != 0 {
                    fs::create_dir_all(config_path.parent().unwrap())?;
                }
                serde_json::to_writer(File::create(config_path)?, &conf)?;
            }

            Commands::Auth(AuthCommands::Whoami {}) => {
                println!("{}", conf.hippo_username);
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
                println!("IMPORTANT: save this Certificate ID for later - you will need it to update and/or delete the Certificate (for now)");
            }

            Commands::Certificate(CertificateCommands::Remove { id }) => {
                hippo_client.remove_certificate(id.to_owned()).await?;
                println!("Removed Certificate {}", id);
            }

            Commands::Channel(ChannelCommands::Add {}) => {
                todo!("https://github.com/deislabs/hippo/pull/389");
                // let id = hippo_client.add_channel(name.to_owned(), storage_id.to_owned()).await?;
                // println!("Added Channel {} (ID = '{}')", name, id);
                // println!("IMPORTANT: save this Channel ID for later - you will need it to update and/or delete the Channel (for now)");
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
                println!("IMPORTANT: save this Environment Variable ID for later - you will need it to update and/or delete the Environment Variable (for now)");
            }

            Commands::Env(EnvCommands::Remove { id }) => {
                hippo_client
                    .remove_environment_variable(id.to_owned())
                    .await?;
                println!("Removed Environment Variable {}", id);
            }

            Commands::New(NewCommands::Hippofacts {
                name,
                destination,
                authors,
                module,
            }) => {
                let handler = RawHandler {
                    name: Some(module.as_os_str().to_string_lossy().to_string()),
                    entrypoint: None,
                    route: "/".to_owned(),
                    files: None,
                    external: None,
                };
                // if dir is a directory, join with HIPPOFACTS. Otherwise, use it as a file name.
                let md = tokio::fs::metadata(&destination).await?;
                let dest = if md.is_dir() {
                    PathBuf::from(&destination).join("HIPPOFACTS")
                } else {
                    PathBuf::from(&destination)
                };

                // Don't overwrite an existing HIPPOFACTS
                match tokio::fs::metadata(&dest).await {
                    Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
                    Err(e) => Err(e),
                    Ok(_) => {
                        anyhow::bail!(
                            "Cowardly refusing to overwrite file. Remove HIPPOFACTS first."
                        )
                    }
                }?;

                let hippofacts = RawHippoFacts {
                    bindle: BindleSpec {
                        name: name.to_owned(),
                        version: "0.1.0".to_owned(),
                        description: None,
                        authors: Some(authors.to_owned()),
                    },
                    annotations: None,
                    export: None,
                    handler: Some(vec![handler]),
                };

                let data = toml::to_vec(&hippofacts)?;
                tokio::fs::write(dest, data).await?;
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

            Commands::Bindle {
                path,
                invoice_version,
            } => {
                let destination = temp_dir().join("hippo-staging");
                let invoice = prepare(path, invoice_version, &bindle_connection_info, &destination).await?;

                push_all(&destination, &invoice.bindle.id, &bindle_connection_info).await?;
                println!("Pushed {}", &invoice.bindle.id);
            }

            Commands::Prepare {
                path,
                invoice_version,
                destination
            } => {
                let invoice = prepare(path, invoice_version, &bindle_connection_info, destination).await?;

                println!("Wrote {} to {}", &invoice.bindle.id, destination.as_os_str().to_string_lossy());
            }

            Commands::Push {
                path,
                invoice_version,
            } => {
                let destination = temp_dir().join("hippo-staging");
                let invoice = prepare(path, invoice_version, &bindle_connection_info, &destination).await?;

                push_all(&destination, &invoice.bindle.id, &bindle_connection_info).await?;
                hippo_client
                    .add_revision(
                        invoice.bindle.id.name().to_string(),
                        invoice.bindle.id.version_string(),
                    )
                    .await?;

                println!("Pushed {}", &invoice.bindle.id);
            }
        }

        Ok(())
    }
}

async fn prepare(path: &PathBuf, invoice_version: &String, bindle_connection_info: &BindleConnectionInfo, destination: &PathBuf) -> Result<bindle::Invoice, anyhow::Error> {
    let path = current_dir()?.join(path);
    let invoice_versioning = InvoiceVersioning::parse(invoice_version)?;
    let hippofacts_filepath = hippofacts_file_path(path.to_path_buf())?;
    let spec = HippoFacts::read_from(hippofacts_filepath)?;
    let external_invoices =
        prefetch_required_invoices(&spec, Some(bindle_connection_info)).await?;
    let expansion_context = ExpansionContext {
        relative_to: path.clone(),
        invoice_versioning,
        external_invoices,
    };
    let (invoice, warnings) =
        crate::expander::expand(&spec, &expansion_context)?.into();
    for warning in &warnings {
        warn!("{}", warning);
    }
    let writer = Writer::new(&path, &destination);
    writer.write(&invoice).await?;
    Ok(invoice)
}

fn hippofacts_file_path(source: PathBuf) -> anyhow::Result<PathBuf> {
    if source.is_dir() {
        find_hippofacts_file_in(&source)
    } else if source.is_file() {
        Ok(source)
    } else {
        Err(anyhow::anyhow!(
            "Artifacts spec not found: file {} does not exist",
            source.to_string_lossy()
        ))
    }
}

// The list of filenames to look for has to take case sensitivity
// into account.
#[cfg(target_os = "windows")]
const SPEC_FILENAMES: &[&str] = &["HIPPOFACTS", "hippofacts.toml"];
// TODO: apparently there is a config option to make Mac filesystems
// case sensitive; fml.
#[cfg(target_os = "macos")]
const SPEC_FILENAMES: &[&str] = &["HIPPOFACTS", "hippofacts.toml"];
#[cfg(target_os = "linux")]
const SPEC_FILENAMES: &[&str] = &[
    "HIPPOFACTS",
    "hippofacts",
    "Hippofacts",
    "HIPPOFACTS.toml",
    "hippofacts.toml",
    "Hippofacts.toml",
];

fn find_hippofacts_file_in(source_dir: &PathBuf) -> anyhow::Result<PathBuf> {
    let candidates = SPEC_FILENAMES
        .iter()
        .flat_map(|f| {
            let source = source_dir.join(f);
            if source.is_file() {
                Some(source)
            } else {
                None
            }
        })
        .collect_vec();

    match candidates.len() {
        0 => Err(anyhow::anyhow!(
            "No artifacts spec not found in directory {}: create a HIPPOFACTS file",
            source_dir.to_string_lossy()
        )),
        1 => Ok(candidates[0].clone()),
        _ => Err(anyhow::anyhow!(
            "Multiple artifacts specs found in directory {}: pass a specific file",
            source_dir.to_string_lossy()
        )),
    }
}

/// Pre-fetch any invoices that are referenced in the HIPPOFACTS.
async fn prefetch_required_invoices(
    hippofacts: &HippoFacts,
    bindle_client_factory: Option<&BindleConnectionInfo>,
) -> anyhow::Result<HashMap<bindle::Id, bindle::Invoice>> {
    let mut map = HashMap::new();

    let external_refs: Vec<bindle::Id> = hippofacts
        .entries
        .iter()
        .flat_map(external_bindle_id)
        .collect();
    if external_refs.is_empty() {
        return Ok(map);
    }

    let client = bindle_client_factory
        .as_ref()
        .ok_or_else(|| {
            anyhow::anyhow!(
                "Spec file contains external references but Bindle server URL is not set"
            )
        })?
        .client()?;

    for external_ref in external_refs {
        let invoice = client
            .get_yanked_invoice(&external_ref)
            .await
            .map_err(|e| {
                anyhow::anyhow!(
                    "Error retrieving external reference {}: {}",
                    external_ref,
                    e
                )
            })?;
        map.insert(external_ref, invoice);
    }

    Ok(map)
}

/// Calculate the external Bindle ID from hippofacts data.
fn external_bindle_id(entry: &HippoFactsEntry) -> Option<bindle::Id> {
    entry.external_ref().map(|ext| ext.bindle_id)
}
