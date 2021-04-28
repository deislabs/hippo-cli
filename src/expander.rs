use std::path::PathBuf;

use glob::GlobError;

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
    paths.into_iter().map(expand_one_match_to_parcel).collect()
}

fn expand_one_match_to_parcel(path: Result<PathBuf, GlobError>) -> anyhow::Result<Parcel> {
    todo!("expand_one_match_to_parcel");
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
