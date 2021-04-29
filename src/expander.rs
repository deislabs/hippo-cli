use std::path::PathBuf;

use glob::GlobError;
use sha2::{Digest, Sha256};

use crate::HippoFacts;
use crate::invoice::{BindleSpec, Invoice, Label, Parcel};

pub fn expand(hippofacts: HippoFacts) -> anyhow::Result<Invoice> {
    let parcels = expand_all_files_to_parcels(&hippofacts)?;

    let invoice = Invoice {
        bindle_version: "1.0.0".to_owned(),
        yanked: None,
        bindle: BindleSpec {
            name: hippofacts.bindle.name.clone(),
            version: hippofacts.bindle.version.clone(), // TODO: mangle by default
            description: hippofacts.bindle.description.clone(),
            authors: hippofacts.bindle.authors.clone(),
        },
        annotations: hippofacts.annotations.clone(),
        parcel: Some(parcels),
        group: None,
    };

    Ok(invoice)
}

fn expand_all_files_to_parcels(hippofacts: &HippoFacts) -> anyhow::Result<Vec<Parcel>> {
    let f = hippofacts.files.iter()
              .map(|(_, v)| expand_files_to_parcels(&v));
    flatten_or_fail(f)
}

fn expand_files_to_parcels(patterns: &Vec<String>) -> anyhow::Result<Vec<Parcel>> {
    let f = patterns.iter()
            .map(expand_file_to_parcels);
    flatten_or_fail(f)
}

fn expand_file_to_parcels(pattern: &String) -> anyhow::Result<Vec<Parcel>> {
    let paths = glob::glob(pattern)?;
    paths.into_iter().map(try_convert_one_match_to_parcel).collect()
}

fn try_convert_one_match_to_parcel(path: Result<PathBuf, GlobError>) -> anyhow::Result<Parcel> {
    match path {
        Err(e) => Err(anyhow::Error::new(e)),
        Ok(path) => convert_one_match_to_parcel(path),
    }
}

fn convert_one_match_to_parcel(path: PathBuf) -> anyhow::Result<Parcel> {
    let mut file = std::fs::File::open(&path)?;

    let name = path.to_str().ok_or(anyhow::Error::msg("Unable to stringise path"))?;
    let size = file.metadata()?.len();

    let mut sha = Sha256::new();
    std::io::copy(&mut file, &mut sha)?;
    let digest_value = sha.finalize();
    let digest_string = format!("{:x}", digest_value);

    let media_type =
        mime_guess::from_path(&path)
            .first_or_octet_stream()
            .to_string();

    Ok(Parcel {
        label: Label {
            name: name.to_owned(),
            sha256: digest_string,
            media_type,
            size,
            ..Label::default()
        },
        conditions: None,
    })
}

fn flatten_or_fail<I, T>(source: I) -> anyhow::Result<Vec<T>>
    where I: IntoIterator<Item = anyhow::Result<Vec<T>>>
{
    let (errs, oks): (Vec<_>, Vec<_>) = source.into_iter().partition(|v| v.is_err());

    for e in errs.into_iter() {
        return e;
    }

    let elements = oks.into_iter().map(|v| v.unwrap()).flatten();
    Ok(elements.collect())
}
