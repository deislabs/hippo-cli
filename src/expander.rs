use std::path::{Path, PathBuf};

use glob::GlobError;
use sha2::{Digest, Sha256};

use crate::invoice::{BindleSpec, Invoice, Label, Parcel};
use crate::HippoFacts;

pub struct ExpansionContext {
    pub relative_to: PathBuf,
    pub invoice_versioning: InvoiceVersioning,
}

impl ExpansionContext {
    pub fn to_absolute(&self, pattern: &str) -> String {
        let absolute = self.relative_to.join(pattern);
        absolute.to_string_lossy().to_string()
    }

    pub fn to_relative(&self, path: impl AsRef<Path>) -> anyhow::Result<String> {
        let relative_path = path.as_ref().strip_prefix(&self.relative_to)?;
        let relative_path_string = relative_path
            .to_str()
            .ok_or(anyhow::Error::msg("Can't convert back to relative path"))?
            .to_owned();
        Ok(relative_path_string)
    }

    pub fn mangle_version(&self, version: &str) -> String {
        match self.invoice_versioning {
            InvoiceVersioning::Dev => version.to_owned(),
            InvoiceVersioning::Production => version.to_owned(),
        }
    }
}


pub enum InvoiceVersioning {
    Dev,
    Production,
}

impl InvoiceVersioning {
    pub fn parse(text: &str) -> Self {
        if text == "production" {
            InvoiceVersioning::Production
        } else {
            InvoiceVersioning::Dev
        }
    }
}

pub fn expand(
    hippofacts: &HippoFacts,
    expansion_context: &ExpansionContext,
) -> anyhow::Result<Invoice> {
    let parcels = expand_all_files_to_parcels(&hippofacts, expansion_context)?;

    let invoice = Invoice {
        bindle_version: "1.0.0".to_owned(),
        yanked: None,
        bindle: BindleSpec {
            name: hippofacts.bindle.name.clone(),
            version: expansion_context.mangle_version(&hippofacts.bindle.version),
            description: hippofacts.bindle.description.clone(),
            authors: hippofacts.bindle.authors.clone(),
        },
        annotations: hippofacts.annotations.clone(),
        parcel: Some(parcels),
        group: None,
    };

    Ok(invoice)
}

fn expand_all_files_to_parcels(
    hippofacts: &HippoFacts,
    expansion_context: &ExpansionContext,
) -> anyhow::Result<Vec<Parcel>> {
    let parcels = hippofacts
        .files
        .iter()
        .map(|(_, v)| expand_files_to_parcels(&v, expansion_context));
    flatten_or_fail(parcels)
}

fn expand_files_to_parcels(
    patterns: &[String],
    expansion_context: &ExpansionContext,
) -> anyhow::Result<Vec<Parcel>> {
    let parcels = patterns
        .iter()
        .map(|f| expand_file_to_parcels(f, expansion_context));
    flatten_or_fail(parcels)
}

fn expand_file_to_parcels(
    pattern: &str,
    expansion_context: &ExpansionContext,
) -> anyhow::Result<Vec<Parcel>> {
    let paths = glob::glob(&expansion_context.to_absolute(pattern))?;
    paths
        .into_iter()
        .map(|p| try_convert_one_match_to_parcel(p, expansion_context))
        .collect()
}

fn try_convert_one_match_to_parcel(
    path: Result<PathBuf, GlobError>,
    expansion_context: &ExpansionContext,
) -> anyhow::Result<Parcel> {
    match path {
        Err(e) => Err(anyhow::Error::new(e)),
        Ok(path) => convert_one_match_to_parcel(path, expansion_context),
    }
}

fn convert_one_match_to_parcel(
    path: PathBuf,
    expansion_context: &ExpansionContext,
) -> anyhow::Result<Parcel> {
    let mut file = std::fs::File::open(&path)?;

    // TODO: We probably want this to be a relative path, first for bindle fidelity,
    // and also so we can find the damn file when we go to upload things!
    let name = expansion_context.to_relative(&path)?;
    let size = file.metadata()?.len();

    let mut sha = Sha256::new();
    std::io::copy(&mut file, &mut sha)?;
    let digest_value = sha.finalize();
    let digest_string = format!("{:x}", digest_value);

    let media_type = mime_guess::from_path(&path)
        .first_or_octet_stream()
        .to_string();

    Ok(Parcel {
        label: Label {
            name,
            sha256: digest_string,
            media_type,
            size,
            ..Label::default()
        },
        conditions: None,
    })
}

fn flatten_or_fail<I, T>(source: I) -> anyhow::Result<Vec<T>>
where
    I: IntoIterator<Item = anyhow::Result<Vec<T>>>,
{
    let mut result = vec![];

    for v in source {
        match v {
            Err(e) => return Err(e),
            Ok(mut vals) => result.append(&mut vals),
        }
    }

    Ok(result)
}
