use hippofacts::HippoFacts;

mod expander;
mod hippofacts;
mod invoice;

fn main() -> anyhow::Result<()> {
    let args = std::env::args().skip(1).collect::<Vec<_>>();

    // For simplicity we start out requiring both input and output files -
    // *we will relax this restriction*!
    if args.len() != 2 {
        println!("Usage: hippofactory source-file destination-file");
        return Ok(());
    }

    let source = std::env::current_dir()?.join(&args[0]);
    let destination = std::env::current_dir()?.join(&args[1]);

    run(&source, &destination)
}

fn run(source: impl AsRef<std::path::Path>, destination: impl AsRef<std::path::Path>) -> anyhow::Result<()> {
    let source_dir = source.as_ref().parent().ok_or(anyhow::Error::msg("Can't establish source directory"))?;

    // std::fs::read_to_string(source)
    //     .and_then(|s| toml::from_str(&s))
    //     .and_then(expander::expand)
    //     .and_then(toml::to_string_pretty)
    //     .and_then(|text| std::fs::write(destination, text))?;

    let content = std::fs::read_to_string(&source)?;
    let spec = toml::from_str::<HippoFacts>(&content)?.make_absolute_path(source_dir);
    let invoice = expander::expand(spec)?;
    let invoice_toml = toml::to_string_pretty(&invoice)?;
    std::fs::write(destination, invoice_toml)?;
    Ok(())
}
