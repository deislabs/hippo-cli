use std::convert::TryFrom;
use std::path::{Path, PathBuf};

use bindle::{BindleSpec, Condition, Group, Invoice, Label, Parcel};
use glob::GlobError;
use itertools::Itertools;
use sha2::{Digest, Sha256};

use crate::{HippoFacts, hippofacts::Handler};

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
            .ok_or_else(|| anyhow::Error::msg("Can't convert back to relative path"))?
            .to_owned();
        Ok(relative_path_string)
    }

    pub fn mangle_version(&self, version: &str) -> String {
        match self.invoice_versioning {
            InvoiceVersioning::Dev => {
                let user = current_user()
                    .map(|s| format!("-{}", s))
                    .unwrap_or_else(|| "".to_owned());
                let timestamp = chrono::Local::now()
                    .format("-%Y.%m.%d.%H.%M.%S.%3f")
                    .to_string();
                format!("{}{}{}", version, user, timestamp)
            }
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
    let groups = expand_all_handlers_to_groups(&hippofacts)?;
    let parcels = expand_all_files_to_parcels(&hippofacts, expansion_context)?;

    let invoice = Invoice {
        bindle_version: "1.0.0".to_owned(),
        yanked: None,
        bindle: BindleSpec {
            id: expand_id(&hippofacts.bindle, expansion_context)?,
            description: hippofacts.bindle.description.clone(),
            authors: hippofacts.bindle.authors.clone(),
        },
        annotations: hippofacts.annotations.clone(),
        parcel: Some(parcels),
        group: Some(groups),
        signature: None,
    };

    Ok(invoice)
}

fn expand_id(
    bindle_spec: &crate::hippofacts::BindleSpec,
    expansion_context: &ExpansionContext,
) -> anyhow::Result<bindle::Id> {
    let name = bindle_spec.name.clone();
    let version = expansion_context.mangle_version(&bindle_spec.version);
    let id = bindle::Id::try_from(format!("{}/{}", &name, &version))?;
    Ok(id)
}

fn expand_all_handlers_to_groups(hippofacts: &HippoFacts) -> anyhow::Result<Vec<Group>> {
    let groups = hippofacts.handler.as_ref().ok_or_else(no_handlers)?.iter().map(expand_to_group).collect();
    Ok(groups)
}

fn expand_to_group(handler: &Handler) -> Group {
    Group {
        name: group_name(handler),
        required: None,
        satisfied_by: None,
    }
}

fn group_name(handler: &Handler) -> String {
    format!("{}-files", handler.name)
}

fn expand_all_files_to_parcels(
    hippofacts: &HippoFacts,
    expansion_context: &ExpansionContext,
) -> anyhow::Result<Vec<Parcel>> {
    let handlers = hippofacts.handler.as_ref().ok_or_else(no_handlers)?;
    let parcel_lists = handlers.iter().map(|handler| expand_files_to_parcels(handler, expansion_context));
    let parcels = flatten_or_fail(parcel_lists)?;
    Ok(merge_memberships(parcels))
}

fn expand_files_to_parcels(
    handler: &Handler,
    expansion_context: &ExpansionContext,
) -> anyhow::Result<Vec<Parcel>> {
    // TODO: the handler file parcel needs a label.requires of the group
    let patterns: Vec<String> = match &handler.files {
        None => vec![handler.name.clone()],
        Some(files) => [vec![handler.name.clone()], files.clone()].concat(),
    };
    let parcels = patterns
        .iter()
        .map(|f| expand_file_to_parcels(f, expansion_context, &group_name(handler)));
    flatten_or_fail(parcels)
}

fn expand_file_to_parcels(
    pattern: &str,
    expansion_context: &ExpansionContext,
    member_of: &str,
) -> anyhow::Result<Vec<Parcel>> {
    let paths = glob::glob(&expansion_context.to_absolute(pattern))?;
    paths
        .into_iter()
        .map(|p| try_convert_one_match_to_parcel(p, expansion_context, member_of))
        .collect()
}

fn try_convert_one_match_to_parcel(
    path: Result<PathBuf, GlobError>,
    expansion_context: &ExpansionContext,
    member_of: &str
) -> anyhow::Result<Parcel> {
    match path {
        Err(e) => Err(anyhow::Error::new(e)),
        Ok(path) => convert_one_match_to_parcel(path, expansion_context, member_of),
    }
}

fn convert_one_match_to_parcel(
    path: PathBuf,
    expansion_context: &ExpansionContext,
    member_of: &str
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
        conditions: Some(Condition {
            member_of: Some(vec![member_of.to_owned()]),
            requires: None
        }),
    })
}

fn merge_memberships(parcels: Vec<Parcel>) -> Vec<Parcel> {
    parcels.into_iter()
           .into_grouping_map_by(|p| p.label.sha256.clone())
           .fold_first(|acc, _key, val| merge_parcel_into(acc, val))
           .values()
           .map(|p| p.clone())  // into_values is not yet stable
           .collect()
}

fn merge_parcel_into(first: Parcel, second: Parcel) -> Parcel {
    Parcel {
        label: first.label,
        conditions: merge_parcel_conditions(first.conditions, second.conditions)
    }
}

fn merge_parcel_conditions(first: Option<Condition>, second: Option<Condition>) -> Option<Condition> {
    match first {
        None => second, // shouldn't happen
        Some(first_condition) =>
            match second {
                None => Some(first_condition),
                Some(second_condition) =>
                    Some(merge_condition_lists(first_condition.clone(), second_condition.clone())),
            }
    }
}

fn merge_condition_lists(first: Condition, second: Condition) -> Condition {
    Condition {
        member_of: merge_lists(first.member_of, second.member_of),
        requires: first.requires,
    }
}

fn merge_lists(first: Option<Vec<String>>, second: Option<Vec<String>>) -> Option<Vec<String>> {
    match (first, second) {
        (None, None) => None,
        (some, None) => some,
        (None, some) => some,
        (Some(list1), Some(list2)) => Some(vec![list1, list2].concat()),
    }
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

fn current_user() -> Option<String> {
    std::env::var("USER")
        .or_else(|_| std::env::var("USERNAME"))
        .ok()
}

fn no_handlers() -> anyhow::Error {
    anyhow::anyhow!("No handlers defined in artifact spec")
}

#[cfg(test)]
mod test {
    use std::str::FromStr;

    use super::*;

    fn test_dir(name: &str) -> PathBuf {
        let test_data_base = PathBuf::from_str(env!("CARGO_MANIFEST_DIR")).unwrap().join("testdata");
        test_data_base.join(name)
    }

    fn read_hippofacts(path: impl AsRef<Path>) -> anyhow::Result<HippoFacts> {
        let toml_text = std::fs::read_to_string(path)?;
        let hippofacts: HippoFacts = toml::from_str(&toml_text)?;
        Ok(hippofacts)
    }

    fn parcel_named<'a>(invoice: &'a Invoice, parcel_name: &str) -> &'a Parcel {
        invoice.parcel.as_ref().unwrap().iter().find(|p| p.label.name == parcel_name).unwrap()
    }

    #[test]
    fn test_expansion() {
        // TODO: this is a bad test - embetteren it
        let app1_dir = test_dir("app1");
        let hippofacts = read_hippofacts(app1_dir.join("HIPPOFACTS")).unwrap();
        let expansion_context = ExpansionContext { relative_to: app1_dir, invoice_versioning: InvoiceVersioning::Production };
        let invoice = expand(&hippofacts, &expansion_context).expect("error expanding");
        assert_eq!(hippofacts.bindle.name, invoice.bindle.id.name());
        assert_eq!(2, invoice.group.as_ref().unwrap().len());
        assert_eq!(1, parcel_named(&invoice, "scripts/ignore.json").conditions.as_ref().unwrap().member_of.as_ref().unwrap().len());
        assert_eq!(2, parcel_named(&invoice, "scripts/real.js").conditions.as_ref().unwrap().member_of.as_ref().unwrap().len());
    }
}
