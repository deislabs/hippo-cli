use expander::ExpansionContext;
use hippofacts::HippoFacts;

mod expander;
mod hippofacts;
mod invoice;

const ARG_HIPPOFACTS: &str = "hippofacts_path";
const ARG_INVOICE: &str = "invoice_path";
const ARG_VERSIONING: &str = "versioning";

fn main() -> anyhow::Result<()> {
    let args = clap::App::new("hippofactory")
        .version("0.0.1")
        .author("Deis Labs")
        .arg(clap::Arg::new(ARG_HIPPOFACTS).required(true).index(1).about("The artifacts spec"))
        .arg(clap::Arg::new(ARG_INVOICE).required(true).index(2).about("The invoice file to generate"))
        .arg(clap::Arg::new(ARG_VERSIONING).possible_values(&["dev", "production"]).default_value("dev").required(false).short('v').long("invoice-version").about("How to version the generated invoice"))
        .get_matches();

    let hippofacts_arg = args.value_of(ARG_HIPPOFACTS).ok_or(anyhow::Error::msg("HIPPOFACTS file is required"))?;
    let invoice_arg = args.value_of(ARG_INVOICE).ok_or(anyhow::Error::msg("Invoice path is required"))?;
    let versioning_arg = args.value_of(ARG_VERSIONING).unwrap();

    let source = std::env::current_dir()?.join(hippofacts_arg);
    let destination = std::env::current_dir()?.join(invoice_arg);
    let _invoice_versioning = parse_versioning_arg(versioning_arg);

    run(&source, &destination)
}

fn run(
    source: impl AsRef<std::path::Path>,
    destination: impl AsRef<std::path::Path>,
) -> anyhow::Result<()> {
    let source_dir = source
        .as_ref()
        .parent()
        .ok_or_else(|| anyhow::Error::msg("Can't establish source directory"))?
        .to_path_buf();
    let expansion_context = ExpansionContext {
        relative_to: source_dir,
    };

    // std::fs::read_to_string(source)
    //     .and_then(|s| toml::from_str(&s))
    //     .and_then(expander::expand)
    //     .and_then(toml::to_string_pretty)
    //     .and_then(|text| std::fs::write(destination, text))?;

    let content = std::fs::read_to_string(&source)?;
    let spec = toml::from_str::<HippoFacts>(&content)?;
    let invoice = expander::expand(&spec, &expansion_context)?;
    let invoice_toml = toml::to_string_pretty(&invoice)?;
    std::fs::write(destination, invoice_toml)?;
    Ok(())
}

enum InvoiceVersioning {
    Dev,
    Production,
}

fn parse_versioning_arg(text: &str) -> InvoiceVersioning {
    if text == "production" {
        InvoiceVersioning::Production
    } else {
        InvoiceVersioning::Dev
    }
}
