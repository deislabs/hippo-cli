use bindle_writer::BindleWriter;
use expander::{ExpansionContext, InvoiceVersioning};
use hippofacts::HippoFacts;

mod bindle_writer;
mod expander;
mod hippofacts;
mod invoice;

const ARG_HIPPOFACTS: &str = "hippofacts_path";
const ARG_STAGING_DIR: &str = "staging_dir";
const ARG_VERSIONING: &str = "versioning";

#[async_std::main]
async fn main() -> anyhow::Result<()> {
    let args = clap::App::new("hippofactory")
        .version("0.0.1")
        .author("Deis Labs")
        .arg(
            clap::Arg::new(ARG_HIPPOFACTS)
                .required(true)
                .index(1)
                .about("The artifacts spec"),
        )
        .arg(
            clap::Arg::new(ARG_STAGING_DIR)
                .required(true)
                .index(2)
                .about("The path to stage the artifacts to"),
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
        .get_matches();

    let hippofacts_arg = args
        .value_of(ARG_HIPPOFACTS)
        .ok_or_else(|| anyhow::Error::msg("HIPPOFACTS file is required"))?;
    let invoice_arg = args
        .value_of(ARG_STAGING_DIR)
        .ok_or_else(|| anyhow::Error::msg("Staging directory is required"))?;
    let versioning_arg = args.value_of(ARG_VERSIONING).unwrap();

    let source = std::env::current_dir()?.join(hippofacts_arg);
    let destination = std::env::current_dir()?.join(invoice_arg).canonicalize()?;
    let invoice_versioning = InvoiceVersioning::parse(versioning_arg);

    run(&source, &destination, invoice_versioning).await
}

async fn run(
    source: impl AsRef<std::path::Path>,
    destination: impl AsRef<std::path::Path>,
    invoice_versioning: InvoiceVersioning,
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

    println!("id:      {}/{}", &invoice.bindle.name, &invoice.bindle.version);
    println!("command: bindle push -p {} {}/{}", &destination.as_ref().to_string_lossy(), &invoice.bindle.name, &invoice.bindle.version);

    Ok(())
}
