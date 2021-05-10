use bindle_writer::BindleWriter;
use expander::{ExpansionContext, InvoiceVersioning};
use hippofacts::HippoFacts;

mod bindle_pusher;
mod bindle_writer;
mod expander;
mod hippofacts;

const ARG_HIPPOFACTS: &str = "hippofacts_path";
const ARG_STAGING_DIR: &str = "output_dir";
const ARG_OUTPUT: &str = "output_format";
const ARG_VERSIONING: &str = "versioning";
const ARG_SERVER_URL: &str = "bindle_server";
const ARG_PREPARE_ONLY: &str = "prepare";

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = clap::App::new(env!("CARGO_PKG_NAME"))
        .version(env!("CARGO_PKG_VERSION"))
        .author("Deis Labs")
        .about("Expands Hippo artifacts files for upload to application storage")
        .arg(
            clap::Arg::new(ARG_HIPPOFACTS)
                .required(true)
                .index(1)
                .about("The artifacts spec (file or directory containing HIPPOFACTS file)"),
        )
        .arg(
            clap::Arg::new(ARG_STAGING_DIR)
                .required(false)
                .takes_value(true)
                .short('d')
                .long("dir")
                .about("The path to output the artifacts to (required with --prepare, otherwise uses a temporary directory)"),
        )
        .arg(
            clap::Arg::new(ARG_VERSIONING)
                .possible_values(&["dev", "production"])
                .default_value("dev")
                .required(false)
                .short('v')
                .long("invoice-version")
                .about("How to version the generated invoice"),
        )
        .arg(
            clap::Arg::new(ARG_OUTPUT)
                .possible_values(&["id", "message", "none"])
                .default_value("message")
                .required(false)
                .short('o')
                .long("output")
                .about("What to print on success"),
        )
        .arg(
            clap::Arg::new(ARG_SERVER_URL)
                .required_unless_present(ARG_PREPARE_ONLY)
                .short('s')
                .long("server")
                .env("BINDLE_SERVER_URL")
                .about("The Bindle server to push the artifacts to")
        )
        .arg(
            clap::Arg::new(ARG_PREPARE_ONLY)
                .required(false)
                .requires(ARG_STAGING_DIR)
                .takes_value(false)
                .long("prepare")
                .about("Prepares an artifact layout in <output_dir> but does not push"),
        )
        .get_matches();

    let hippofacts_arg = args
        .value_of(ARG_HIPPOFACTS)
        .ok_or_else(|| anyhow::Error::msg("HIPPOFACTS file is required"))?;
    let staging_dir_arg = args
        .value_of(ARG_STAGING_DIR);
    let versioning_arg = args.value_of(ARG_VERSIONING).unwrap();
    let output_format_arg = args.value_of(ARG_OUTPUT).unwrap();
    let push_to =
        if args.is_present(ARG_PREPARE_ONLY) {
            None
        } else {
            Some(args.value_of(ARG_SERVER_URL).ok_or_else(|| anyhow::Error::msg("Server URL is required"))?.to_owned())
        };

    let source_file_or_dir = std::env::current_dir()?.join(hippofacts_arg);
    let source = if source_file_or_dir.is_file() {
        source_file_or_dir
    } else {
        source_file_or_dir.join("HIPPOFACTS")
    };
    if !source.exists() {
        return Err(anyhow::anyhow!("Artifacts spec not found: file {} does not exist", source.to_string_lossy()));
    }

    let destination = match staging_dir_arg {
        Some(dir) => std::env::current_dir()?.join(dir),
        None => std::env::temp_dir().join("hippo-staging"),  // TODO: make unpredictable?
    };
    let invoice_versioning = InvoiceVersioning::parse(versioning_arg);
    let output_format = OutputFormat::parse(output_format_arg);

    run(&source, &destination, invoice_versioning, output_format, push_to).await
}

async fn run(
    source: impl AsRef<std::path::Path>,
    destination: impl AsRef<std::path::Path>,
    invoice_versioning: InvoiceVersioning,
    output_format: OutputFormat,
    push_to: Option<String>,
) -> anyhow::Result<()> {
    let source_dir = source
        .as_ref()
        .parent()
        .ok_or_else(|| anyhow::Error::msg("Can't establish source directory"))?
        .to_path_buf();
    let expansion_context = ExpansionContext {
        relative_to: source_dir.clone(),
        invoice_versioning,
    };
    let writer = BindleWriter::new(&source_dir, &destination);

    let content = std::fs::read_to_string(&source)?;
    let spec = toml::from_str::<HippoFacts>(&content)?;
    let invoice = expander::expand(&spec, &expansion_context)?;
    writer.write(&invoice).await?;

    if let Some(url) = &push_to {
        bindle_pusher::push_all(&destination, &invoice.bindle.id, &url).await?;
    }

    match output_format {
        OutputFormat::None => (),
        OutputFormat::Id => println!("{}", &invoice.bindle.id),
        OutputFormat::Message => {
            if push_to.is_some() {
                println!("pushed: {}", &invoice.bindle.id);
            } else {
                println!("id:      {}", &invoice.bindle.id);
                println!("command: bindle push -p {} {}", &destination.as_ref().canonicalize()?.to_string_lossy(), &invoice.bindle.id);
            }
        }
    }

    Ok(())
}

enum OutputFormat {
    None,
    Id,
    Message,
}

impl OutputFormat {
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
