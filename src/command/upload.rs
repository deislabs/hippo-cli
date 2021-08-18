use async_trait::async_trait;
use clap::{App, Arg, ArgMatches};
use colored::Colorize;
use itertools::Itertools;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::bindle_utils::BindleConnectionInfo;
use crate::bindle_writer::BindleWriter;
use crate::expander::{ExpansionContext, InvoiceVersioning};
use crate::hippo_notifier::ConnectionInfo;
use crate::hippofacts::{HippoFacts, HippoFactsEntry};

const ARG_HIPPOFACTS: &str = "hippofacts_path";
const ARG_STAGING_DIR: &str = "output_dir";
const ARG_OUTPUT: &str = "output_format";
const ARG_VERSIONING: &str = "versioning";
const ARG_BINDLE_URL: &str = "bindle_server";
const ARG_HIPPO_URL: &str = "hippo_url";
const ARG_HIPPO_USERNAME: &str = "hippo_username";
const ARG_HIPPO_PASSWORD: &str = "hippo_password";
const ARG_INSECURE: &str = "insecure";

pub(crate) const CMD_BINDLE: &str = "bindle";
pub(crate) const CMD_PUSH: &str = "push";
pub(crate) const CMD_PREPARE: &str = "prepare";

// Arguments necessary to build a bindle (but not push it)
// - ARG_HIPPOFACTS
// - ARG_VERSIONING
// - ARG_OUTPUT
// - ARG_BINDLE_URL
// - ARG_STAGING_DIR
fn common_args<'a>(cmd: &str) -> Vec<Arg<'a>> {
    let (require_bindle_server, require_stage_dir) = match cmd {
        CMD_BINDLE => (true, false),
        CMD_PREPARE => (false, true),
        CMD_PUSH => (true, false),
        _ => (false, false),
    };

    vec![
        Arg::new(ARG_HIPPOFACTS)
            .required(true)
            .index(1)
            .about("The artifacts spec (file or directory containing HIPPOFACTS file)"),
        Arg::new(ARG_VERSIONING)
            .possible_values(&["dev", "production"])
            .default_value("dev")
            .required(false)
            .short('v')
            .long("invoice-version")
            .about("How to version the generated invoice"),
        Arg::new(ARG_OUTPUT)
            .possible_values(&["id", "message", "none"])
            .default_value("message")
            .required(false)
            .short('o')
            .long("output")
            .about("What to print on success"),
        Arg::new(ARG_BINDLE_URL)
            .short('s')
            .long("server")
            .env("BINDLE_URL")
            .about("The Bindle server to push the artifacts to")
            .required(require_bindle_server),
        Arg::new(ARG_INSECURE)
            .required(false)
            .takes_value(false)
            .short('k')
            .long("insecure")
            .about("If set, ignore server certificate errors"),
        Arg::new(ARG_STAGING_DIR)
            .takes_value(true)
            .short('d')
            .long("dir")
            .about("The path to output the artifacts to. Required when doing a `hippo prepare`. Other commands will use a temp dir if this is not specified.")
            .required(require_stage_dir),
    ]
}

/// Push a bindle to the Bindle server, then notify Hippo to load the new version.
pub(crate) struct Push;
#[async_trait]
impl super::CommandRunner for Push {
    fn app<'a>() -> App<'a> {
        App::new(CMD_PUSH)
            .about("Packages and uploads Hippo artifacts, notifying Hippo")
            .alias("ship")
            .args(common_args(CMD_PUSH))
            .arg(
                Arg::new(ARG_HIPPO_URL)
                    .long("hippo-url")
                    .env("HIPPO_URL")
                    .about("The Hippo service to push the artifacts to")
                    .required(true),
            )
            .arg(
                Arg::new(ARG_HIPPO_USERNAME)
                    .long("hippo-username")
                    .env("HIPPO_USERNAME")
                    .about("The username for connecting to Hippo")
                    .required(true),
            )
            .arg(
                Arg::new(ARG_HIPPO_PASSWORD)
                    .long("hippo-password")
                    .env("HIPPO_PASSWORD")
                    .about("The username for connecting to Hippo")
                    .required(true),
            )
    }
    /// Package a bindle and push it to a Bindle server, notifying Hippo.
    async fn run(&self, args: &ArgMatches) -> anyhow::Result<()> {
        let source = hippofacts_file_path_from_args(args)?;

        // Local configuration
        let versioning_arg = args.value_of(ARG_VERSIONING).unwrap();
        let output_format_arg = args.value_of(ARG_OUTPUT).unwrap();
        let destination = match args.value_of(ARG_STAGING_DIR) {
            Some(dir) => std::env::current_dir()?.join(dir),
            None => std::env::temp_dir().join("hippo-staging"), // TODO: make unpredictable with tempdir?
        };
        let invoice_versioning = InvoiceVersioning::parse(versioning_arg);
        let output_format = OutputFormat::parse(output_format_arg);

        // Bindle configuration
        let bindle_settings = BindleSettings::Push(
            BindleConnectionInfo::from_args(args).ok_or_else(bindle_url_is_required)?,
        );

        // Hippo configuration
        let hippo_url = args
            .value_of(ARG_HIPPO_URL)
            .map(|s| s.to_owned())
            .ok_or_else(|| {
                anyhow::anyhow!("A Hippo url is required. Use --hippo-url or $HIPPO_URL")
            })?;
        let hippo_username = args.value_of(ARG_HIPPO_USERNAME);
        let hippo_password = args.value_of(ARG_HIPPO_PASSWORD);

        // Notification configuration
        let notify_to = Some(ConnectionInfo {
            url: hippo_url,
            danger_accept_invalid_certs: args.is_present(ARG_INSECURE),
            username: hippo_username.unwrap().to_owned(), // Known to be set if the URL is
            password: hippo_password.unwrap().to_owned(),
        });

        run(
            &source,
            &destination,
            invoice_versioning,
            output_format,
            bindle_settings,
            notify_to,
        )
        .await
    }
}

/// Send a bindle to a Bindle server, but don't notify Hippo.
pub(crate) struct Bindle;
#[async_trait]
impl super::CommandRunner for Bindle {
    fn app<'a>() -> App<'a> {
        App::new(CMD_BINDLE)
            .about("Creates a bindle and pushes it to the Bindle server, but does not notify Hippo")
            .args(common_args(CMD_BINDLE))
    }
    /// Run the bindle command
    ///
    /// Args:
    /// - ARG_HIPPOFACTS
    /// - ARG_STAGING_DIR
    /// - ARG_VERSIONING
    /// - ARG_OUTPUT
    /// - ARG_BINDLE_URL
    async fn run(&self, args: &ArgMatches) -> anyhow::Result<()> {
        let source = hippofacts_file_path_from_args(args)?;

        let destination = match args.value_of(ARG_STAGING_DIR) {
            Some(dir) => std::env::current_dir()?.join(dir),
            None => std::env::temp_dir().join("hippo-staging"), // TODO: make unpredictable with tempdir?
        };

        let invoice_versioning = InvoiceVersioning::parse(args.value_of(ARG_VERSIONING).unwrap());
        let output_format = OutputFormat::parse(args.value_of(ARG_OUTPUT).unwrap());
        let bindle_settings = BindleSettings::Push(
            BindleConnectionInfo::from_args(args).ok_or_else(bindle_url_is_required)?,
        );

        run(
            &source,
            &destination,
            invoice_versioning,
            output_format,
            bindle_settings,
            None, // `bindle` never notifies.
        )
        .await
    }
}

/// Prepare a bindle, but write it to disk instead of sending it over the network.
pub(crate) struct Prepare;
#[async_trait]
impl super::CommandRunner for Prepare {
    fn app<'a>() -> App<'a> {
        App::new(CMD_PREPARE)
            .about("Reads a HIPPOFACTS file and prepares a Bindle, caching it locally.")
            .args(common_args(CMD_PREPARE))
    }
    /// Run the prepare command
    ///
    /// Args:
    /// - ARG_HIPPOFACTS
    /// - ARG_STAGING_DIR
    /// - ARG_VERSIONING
    /// - ARG_OUTPUT
    /// - ARG_BINDLE_URL
    async fn run(&self, args: &ArgMatches) -> anyhow::Result<()> {
        let source = hippofacts_file_path_from_args(args)?;

        let current_dir = std::env::current_dir()?;
        let destination = args
            .value_of(ARG_STAGING_DIR)
            .map(|dir| current_dir.join(dir))
            .ok_or_else(|| {
                anyhow::Error::msg("A staging directory is required for 'prepare'. Use -d|--dir.")
            })?;

        let invoice_versioning = InvoiceVersioning::parse(args.value_of(ARG_VERSIONING).unwrap());
        let output_format = OutputFormat::parse(args.value_of(ARG_OUTPUT).unwrap());

        // NOTE: Prepare currently does not require a Bindle URL, so this could be NoPush(None)
        let bindle_settings = BindleSettings::NoPush(BindleConnectionInfo::from_args(args));

        run(
            &source,
            &destination,
            invoice_versioning,
            output_format,
            bindle_settings,
            None, // Prepare never notifies.
        )
        .await
    }
}

/// Run a command to package and push an app, and then notify if necessary.
/// This is used for prepare, bindle, and push commands
/// Run a command to package and push an app, and then notify if necessary.
/// This is used for prepare, bindle, and push commands
async fn run(
    source: impl AsRef<std::path::Path>,
    destination: impl AsRef<std::path::Path>,
    invoice_versioning: InvoiceVersioning,
    output_format: OutputFormat,
    bindle_settings: BindleSettings,
    notify_to: Option<ConnectionInfo>,
) -> anyhow::Result<()> {
    let spec = HippoFacts::read_from(&source)?;

    let source_dir = source
        .as_ref()
        .parent()
        .ok_or_else(|| anyhow::Error::msg("Can't establish source directory"))?
        .to_path_buf();

    // Do this outside the `expand` function so `expand` is more testable
    let external_invoices =
        prefetch_required_invoices(&spec, bindle_settings.connection_info()).await?;

    let expansion_context = ExpansionContext {
        relative_to: source_dir.clone(),
        invoice_versioning,
        external_invoices,
    };

    let (invoice, warnings) = crate::expander::expand(&spec, &expansion_context)?.into();

    for warning in &warnings {
        eprintln!("{}", format!("warning: {}", warning).yellow());
    }

    let writer = BindleWriter::new(&source_dir, &destination);
    writer.write(&invoice).await?;

    if let BindleSettings::Push(bindle_connection) = &bindle_settings {
        crate::bindle_pusher::push_all(&destination, &invoice.bindle.id, bindle_connection).await?;
        if let Some(hippo_connection) = &notify_to {
            crate::hippo_notifier::register(&invoice.bindle.id, hippo_connection).await?;
        }
    }

    // TODO: handle case where push succeeded but notify failed
    match output_format {
        OutputFormat::None => (),
        OutputFormat::Id => println!("{}", &invoice.bindle.id),
        OutputFormat::Message => match &bindle_settings {
            BindleSettings::Push(_) => println!("pushed: {}", &invoice.bindle.id),
            BindleSettings::NoPush(_) => {
                println!("id:      {}", &invoice.bindle.id);
                println!(
                    "command: bindle push -p {} {}",
                    dunce::canonicalize(&destination)?.to_string_lossy(),
                    &invoice.bindle.id
                );
            }
        },
    }

    Ok(())
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
        let invoice = client.get_yanked_invoice(&external_ref).await?;
        map.insert(external_ref, invoice);
    }

    Ok(map)
}

/// Calculate the external Bindle ID from hippofacts data.
fn external_bindle_id(entry: &HippoFactsEntry) -> Option<bindle::Id> {
    entry.external_ref().map(|ext| ext.bindle_id)
}

fn hippofacts_file_path_from_args(args: &ArgMatches) -> anyhow::Result<PathBuf> {
    args.value_of(ARG_HIPPOFACTS)
        .ok_or_else(|| anyhow::Error::msg("HIPPOFACTS file is required"))
        .and_then(hippofacts_file_path)
}

fn hippofacts_file_path(hippofacts_arg: &str) -> anyhow::Result<PathBuf> {
    let source = std::env::current_dir()?.join(hippofacts_arg);
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

const SPEC_FILENAMES: &[&str] = &["HIPPOFACTS", "hippofacts.toml"];

fn find_hippofacts_file_in(source_dir: &Path) -> anyhow::Result<PathBuf> {
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

fn bindle_url_is_required() -> anyhow::Error {
    anyhow::anyhow!("Bindle URL is required. Use -s|--server or $BINDLE_URL")
}

/// Describe the desired output format.
enum OutputFormat {
    None,
    Id,
    Message,
}

impl OutputFormat {
    /// Parse a format from a string.
    pub fn parse(text: &str) -> Self {
        if text == "none" {
            OutputFormat::None
        } else if text == "id" {
            OutputFormat::Id
        } else {
            OutputFormat::Message
        }
    }
}

/// Desribe the actions to be taken viz a viz a Bindle server.
enum BindleSettings {
    /// Do not push to a Bindle server, but still resolve local references.
    NoPush(Option<BindleConnectionInfo>),
    /// Push to a Bindle server, resolving references if necessary.
    Push(BindleConnectionInfo),
}

impl BindleSettings {
    /// Get the Bindle server URL if it was set.
    pub fn connection_info(&self) -> Option<&BindleConnectionInfo> {
        match self {
            Self::NoPush(opt) => opt.as_ref(),
            Self::Push(conn) => Some(conn),
        }
    }
}

impl BindleConnectionInfo {
    pub fn from_args(args: &ArgMatches) -> Option<Self> {
        let allow_insecure = args.is_present(ARG_INSECURE);
        args.value_of(ARG_BINDLE_URL)
            .map(|base_url| Self::new(base_url, allow_insecure))
    }
}
