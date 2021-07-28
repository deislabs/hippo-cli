use std::collections::HashMap;
use std::path::PathBuf;

use bindle_writer::BindleWriter;
use clap::{App, Arg, ArgMatches};
use expander::{ExpansionContext, InvoiceVersioning};
use hippofacts::{HippoFacts, HippoFactsEntry};

mod bindle_pusher;
mod bindle_utils;
mod bindle_writer;
mod expander;
mod hippo_notifier;
mod hippofacts;

const ARG_HIPPOFACTS: &str = "hippofacts_path";
const ARG_STAGING_DIR: &str = "output_dir";
const ARG_OUTPUT: &str = "output_format";
const ARG_VERSIONING: &str = "versioning";
const ARG_BINDLE_URL: &str = "bindle_server";
const ARG_HIPPO_URL: &str = "hippo_url";
const ARG_HIPPO_USERNAME: &str = "hippo_username";
const ARG_HIPPO_PASSWORD: &str = "hippo_password";
const ARG_INSECURE: &str = "insecure";

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
        .subcommand(
            App::new("push")
                .about("Packages and uploads Hippo artifacts, notifying Hippo")
                .args(bindle_build_args(true, false))
                .arg(
                    Arg::new(ARG_HIPPO_URL)
                        .required(true)
                        .long("hippo-url")
                        .env("HIPPO_URL")
                        .about("The Hippo service to push the artifacts to"),
                )
                .arg(
                    Arg::new(ARG_HIPPO_USERNAME)
                        .long("hippo-username")
                        .env("HIPPO_USERNAME")
                        .about("The username for connecting to Hippo"),
                )
                .arg(
                    Arg::new(ARG_HIPPO_PASSWORD)
                        .long("hippo-password")
                        .env("HIPPO_PASSWORD")
                        .about("The username for connecting to Hippo"),
                ),
        )
        .subcommand(
            App::new("prepare")
                .about("Reads a HIPPOFACTS file and prepares a Bindle, caching it locally.")
                .args(bindle_build_args(false, true)),
        )
        .subcommand(
            App::new("bindle")
                .about("Creates a bindle and pushes it to the Bindle server, but does not notify Hippo")
                .args(bindle_build_args(true, false)),
        )
        .get_matches();

    match matches.subcommand() {
        // Make a vague attempt to keep these in alphabetical order
        Some(("bindle", args)) => bindle(args).await,
        Some(("prepare", args)) => prepare(args).await,
        Some(("push", args)) => push(args).await,
        _ => Err(anyhow::anyhow!("No matching command. Try 'hippo help'")),
    }
}

/// Constructs arguments used to do a Bindle build.
/// Arguments necessary to build a bindle (but not push it)
/// - ARG_HIPPOFACTS
/// - ARG_VERSIONING
/// - ARG_OUTPUT
/// - ARG_BINDLE_URL
/// - ARG_STAGING_DIR
fn bindle_build_args<'a>(require_bindle_server: bool, require_stage_dir: bool) -> Vec<Arg<'a>> {
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

/// Run the prepare command
///
/// Args:
/// - ARG_HIPPOFACTS
/// - ARG_STAGING_DIR
/// - ARG_VERSIONING
/// - ARG_OUTPUT
/// - ARG_BINDLE_URL
async fn prepare(args: &ArgMatches) -> anyhow::Result<()> {
    let source = args
        .value_of(ARG_HIPPOFACTS)
        .ok_or_else(|| anyhow::Error::msg("HIPPOFACTS file is required"))
        .and_then(sourcedir)?;

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
    let bindle_settings =
        BindleSettings::NoPush(args.value_of(ARG_BINDLE_URL).map(|s| s.to_owned()));

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

/// Run the bindle command
///
/// Args:
/// - ARG_HIPPOFACTS
/// - ARG_STAGING_DIR
/// - ARG_VERSIONING
/// - ARG_OUTPUT
/// - ARG_BINDLE_URL
async fn bindle(args: &ArgMatches) -> anyhow::Result<()> {
    let source = args
        .value_of(ARG_HIPPOFACTS)
        .ok_or_else(|| anyhow::Error::msg("HIPPOFACTS file is required"))
        .and_then(sourcedir)?;

    let destination = match args.value_of(ARG_STAGING_DIR) {
        Some(dir) => std::env::current_dir()?.join(dir),
        None => std::env::temp_dir().join("hippo-staging"), // TODO: make unpredictable with tempdir?
    };

    let invoice_versioning = InvoiceVersioning::parse(args.value_of(ARG_VERSIONING).unwrap());
    let output_format = OutputFormat::parse(args.value_of(ARG_OUTPUT).unwrap());
    let bindle_settings = BindleSettings::Push(
        args.value_of(ARG_BINDLE_URL)
            .map(|s| s.to_owned())
            .ok_or_else(|| {
                anyhow::anyhow!("Bindle URL is required. Use -s|--server or $BINDLE_URL")
            })?,
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

/// Package a bindle and push it to a Bindle server, notifying Hippo.
async fn push(args: &ArgMatches) -> anyhow::Result<()> {
    let hippofacts_arg = args
        .value_of(ARG_HIPPOFACTS)
        .ok_or_else(|| anyhow::Error::msg("HIPPOFACTS file is required"))?;
    let source = sourcedir(hippofacts_arg)?;

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
    let bindle_url = args.value_of(ARG_BINDLE_URL).map(|s| s.to_owned());
    let bindle_settings = BindleSettings::Push(bindle_url.ok_or_else(|| {
        anyhow::anyhow!("Bindle URL must be set for this action. Use -s|--server or $BINDLE_URL")
    })?);

    // Hippo configuration
    let hippo_url = args
        .value_of(ARG_HIPPO_URL)
        .map(|s| s.to_owned())
        .ok_or_else(|| anyhow::anyhow!("A Hippo url is required. Use --hippo-url or $HIPPO_URL"))?;
    let hippo_username = args.value_of(ARG_HIPPO_USERNAME);
    let hippo_password = args.value_of(ARG_HIPPO_PASSWORD);

    // Notification configuration
    let notify_to = Some(hippo_notifier::ConnectionInfo {
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

/// Run a command to package and push an app, and then notify if necessary.
/// This is used for prepare, bindle, and push commands
async fn run(
    source: impl AsRef<std::path::Path>,
    destination: impl AsRef<std::path::Path>,
    invoice_versioning: InvoiceVersioning,
    output_format: OutputFormat,
    bindle_settings: BindleSettings,
    notify_to: Option<hippo_notifier::ConnectionInfo>,
) -> anyhow::Result<()> {
    let spec = HippoFacts::read_from(&source)?;

    let source_dir = source
        .as_ref()
        .parent()
        .ok_or_else(|| anyhow::Error::msg("Can't establish source directory"))?
        .to_path_buf();

    // Do this outside the `expand` function so `expand` is more testable
    let external_invoices = prefetch_required_invoices(&spec, bindle_settings.bindle_url()).await?;

    let expansion_context = ExpansionContext {
        relative_to: source_dir.clone(),
        invoice_versioning,
        external_invoices,
    };

    let invoice = expander::expand(&spec, &expansion_context)?;

    let writer = BindleWriter::new(&source_dir, &destination);
    writer.write(&invoice).await?;

    if let BindleSettings::Push(url) = &&bindle_settings {
        bindle_pusher::push_all(&destination, &invoice.bindle.id, &url).await?;
        if let Some(hippo_url) = &notify_to {
            hippo_notifier::register(&invoice.bindle.id, &hippo_url).await?;
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
    bindle_url: Option<String>,
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

    let base_url = bindle_url.as_ref().ok_or_else(|| {
        anyhow::anyhow!("Spec file contains external references but Bindle server URL is not set")
    })?;
    let client = bindle::client::Client::new(base_url)?;

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

/// Find the source directory
fn sourcedir(hippofacts_arg: &str) -> anyhow::Result<PathBuf> {
    let source_file_or_dir = std::env::current_dir()?.join(hippofacts_arg);
    let source = if source_file_or_dir.is_file() {
        source_file_or_dir
    } else {
        source_file_or_dir.join("HIPPOFACTS")
    };

    if !source.exists() {
        return Err(anyhow::anyhow!(
            "Artifacts spec not found: file {} does not exist",
            source.to_string_lossy()
        ));
    }
    Ok(source)
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
    NoPush(Option<String>),
    /// Push to a Bindle server, resolving references if necessary.
    Push(String),
}

impl BindleSettings {
    /// Get the Bindle server URL if it was set.
    pub fn bindle_url(&self) -> Option<String> {
        match self {
            Self::NoPush(opt) => opt.clone(),
            Self::Push(url) => Some(url.clone()),
        }
    }
}
