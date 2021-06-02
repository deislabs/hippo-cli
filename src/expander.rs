use std::collections::BTreeMap;
use std::convert::TryFrom;
use std::iter::FromIterator;
use std::path::{Path, PathBuf};

use bindle::{BindleSpec, Condition, Group, Invoice, Label, Parcel};
use glob::GlobError;
use itertools::Itertools;
use sha2::{Digest, Sha256};

use crate::{hippofacts::Handler, HippoFacts};

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
            .to_owned()
            .replace("\\", "/"); // TODO: a better way
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
    let handler_parcels = expand_handler_modules_to_parcels(&hippofacts, expansion_context)?;
    let file_parcels = expand_all_files_to_parcels(&hippofacts, expansion_context)?;
    let parcels = handler_parcels.into_iter().chain(file_parcels).collect();

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
    let groups = hippofacts
        .handler
        .as_ref()
        .ok_or_else(no_handlers)?
        .iter()
        .map(expand_to_group)
        .collect();
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

fn expand_handler_modules_to_parcels(
    hippofacts: &HippoFacts,
    expansion_context: &ExpansionContext,
) -> anyhow::Result<Vec<Parcel>> {
    let handlers = hippofacts.handler.as_ref().ok_or_else(no_handlers)?;
    let parcels = handlers.iter().map(|handler| {
        convert_one_match_to_parcel(
            PathBuf::from(expansion_context.to_absolute(&handler.name)),
            expansion_context,
            vec![("route", &handler.route), ("file", "false")],
            None,
            Some(&group_name(handler)),
        )
    });
    parcels.collect()
}

fn expand_all_files_to_parcels(
    hippofacts: &HippoFacts,
    expansion_context: &ExpansionContext,
) -> anyhow::Result<Vec<Parcel>> {
    let handlers = hippofacts.handler.as_ref().ok_or_else(no_handlers)?;
    let parcel_lists = handlers
        .iter()
        .map(|handler| expand_files_to_parcels(handler, expansion_context));
    let parcels = flatten_or_fail(parcel_lists)?;
    Ok(merge_memberships(parcels))
}

fn expand_files_to_parcels(
    handler: &Handler,
    expansion_context: &ExpansionContext,
) -> anyhow::Result<Vec<Parcel>> {
    let patterns: Vec<String> = match &handler.files {
        None => vec![],
        Some(files) => files.clone(),
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
    member_of: &str,
) -> anyhow::Result<Parcel> {
    match path {
        Err(e) => Err(anyhow::Error::new(e)),
        Ok(path) => {
            let features = vec![("file", "true")];
            convert_one_match_to_parcel(path, expansion_context, features, Some(member_of), None)
        }
    }
}

fn convert_one_match_to_parcel(
    path: PathBuf,
    expansion_context: &ExpansionContext,
    wagi_features: Vec<(&str, &str)>,
    member_of: Option<&str>,
    requires: Option<&str>,
) -> anyhow::Result<Parcel> {
    let mut file = std::fs::File::open(&path)?;

    let name = expansion_context.to_relative(&path)?;
    let size = file.metadata()?.len();

    let mut sha = Sha256::new();
    std::io::copy(&mut file, &mut sha)?;
    let digest_value = sha.finalize();
    let digest_string = format!("{:x}", digest_value);

    let media_type = mime_guess::from_path(&path)
        .first_or_octet_stream()
        .to_string();

    // let features = vec![("route", route)];
    let feature = Some(wagi_feature_of(wagi_features));

    Ok(Parcel {
        label: Label {
            name,
            sha256: digest_string,
            media_type,
            size,
            feature,
            ..Label::default()
        },
        conditions: Some(Condition {
            member_of: vector_of(member_of),
            requires: vector_of(requires),
        }),
    })
}

fn merge_memberships(parcels: Vec<Parcel>) -> Vec<Parcel> {
    parcels
        .into_iter()
        .into_grouping_map_by(file_id)
        .fold_first(|acc, _key, val| merge_parcel_into(acc, val))
        .values()
        .cloned() // into_values is not yet stable
        .collect()
}

fn merge_parcel_into(first: Parcel, second: Parcel) -> Parcel {
    Parcel {
        label: first.label,
        conditions: merge_parcel_conditions(first.conditions, second.conditions),
    }
}

fn merge_parcel_conditions(
    first: Option<Condition>,
    second: Option<Condition>,
) -> Option<Condition> {
    match first {
        None => second, // shouldn't happen
        Some(first_condition) => match second {
            None => Some(first_condition),
            Some(second_condition) => {
                Some(merge_condition_lists(first_condition, second_condition))
            }
        },
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

fn file_id(parcel: &Parcel) -> String {
    // Two parcels with different names could refer to the same content.  We
    // don't want to treat them as the same parcel when deduplicating.
    format!("{}@{}", parcel.label.sha256, parcel.label.name)
}

fn vector_of(option: Option<&str>) -> Option<Vec<String>> {
    option.map(|val| vec![val.to_owned()])
}

fn wagi_feature_of(values: Vec<(&str, &str)>) -> BTreeMap<String, BTreeMap<String, String>> {
    BTreeMap::from_iter(vec![("wagi".to_owned(), feature_map_of(values))])
}

fn feature_map_of(values: Vec<(&str, &str)>) -> BTreeMap<String, String> {
    values
        .into_iter()
        .map(|(k, v)| (k.to_owned(), v.to_owned()))
        .collect()
}

#[cfg(test)]
mod test {
    use std::str::FromStr;

    use super::*;

    fn test_dir(name: &str) -> PathBuf {
        let test_data_base = PathBuf::from_str(env!("CARGO_MANIFEST_DIR"))
            .unwrap()
            .join("testdata");
        test_data_base.join(name)
    }

    fn read_hippofacts(path: impl AsRef<Path>) -> anyhow::Result<HippoFacts> {
        let toml_text = std::fs::read_to_string(path)?;
        let hippofacts: HippoFacts = toml::from_str(&toml_text)?;
        Ok(hippofacts)
    }

    fn parcel_named<'a>(invoice: &'a Invoice, parcel_name: &str) -> &'a Parcel {
        invoice
            .parcel
            .as_ref()
            .unwrap()
            .iter()
            .find(|p| p.label.name == parcel_name)
            .unwrap()
    }

    fn parcel_feature_value<'a>(
        invoice: &'a Invoice,
        parcel_name: &str,
        feature_name: &str,
        item_name: &str,
    ) -> &'a str {
        parcel_named(invoice, parcel_name)
            .label
            .feature
            .as_ref()
            .unwrap()
            .get(feature_name)
            .as_ref()
            .unwrap()
            .get(item_name)
            .as_ref()
            .unwrap()
    }

    fn parcel_conditions<'a>(invoice: &'a Invoice, parcel_name: &str) -> &'a Condition {
        parcel_named(invoice, parcel_name)
            .conditions
            .as_ref()
            .unwrap()
    }

    fn expand_test_invoice(name: &str) -> anyhow::Result<Invoice> {
        let dir = test_dir(name);
        let hippofacts = read_hippofacts(dir.join("HIPPOFACTS")).unwrap();
        let expansion_context = ExpansionContext {
            relative_to: dir,
            invoice_versioning: InvoiceVersioning::Production,
        };
        let invoice = expand(&hippofacts, &expansion_context).expect("error expanding");
        Ok(invoice)
    }

    #[test]
    fn test_name_is_kept() {
        let invoice = expand_test_invoice("app1").unwrap();
        assert_eq!("weather", invoice.bindle.id.name());
    }

    #[test]
    fn test_route_is_mapped() {
        let invoice = expand_test_invoice("app1").unwrap();
        assert_eq!(
            "/fake",
            parcel_feature_value(&invoice, "out/fake.wasm", "wagi", "route")
        );
    }

    #[test]
    fn test_handler_parcel_is_not_asset() {
        let invoice = expand_test_invoice("app1").unwrap();
        assert_eq!(
            "false",
            parcel_feature_value(&invoice, "out/fake.wasm", "wagi", "file")
        );
    }

    #[test]
    fn test_group_is_created_per_handler() {
        let invoice = expand_test_invoice("app1").unwrap();
        let groups = invoice.group.as_ref().unwrap();
        assert_eq!(2, groups.len());
        assert_eq!("out/fake.wasm-files", groups[0].name);
        assert_eq!("out/lies.wasm-files", groups[1].name);
    }

    #[test]
    fn test_files_are_members_of_correct_groups() {
        let invoice = expand_test_invoice("app1").unwrap();
        assert_eq!(
            1,
            parcel_conditions(&invoice, "scripts/ignore.json")
                .member_of
                .as_ref()
                .unwrap()
                .len()
        );
        assert_eq!(
            2,
            parcel_conditions(&invoice, "scripts/real.js")
                .member_of
                .as_ref()
                .unwrap()
                .len()
        );
    }

    #[test]
    fn test_assets_parcels_are_marked_as_assets() {
        let invoice = expand_test_invoice("app1").unwrap();
        assert_eq!(
            "true",
            parcel_feature_value(&invoice, "scripts/real.js", "wagi", "file")
        );
    }

    #[test]
    fn test_handler_parcels_are_not_members_of_groups() {
        let invoice = expand_test_invoice("app1").unwrap();
        assert_eq!(None, parcel_conditions(&invoice, "out/lies.wasm").member_of);
    }

    #[test]
    fn test_handlers_require_correct_groups() {
        let invoice = expand_test_invoice("app1").unwrap();
        assert_eq!(
            1,
            parcel_conditions(&invoice, "out/lies.wasm")
                .requires
                .as_ref()
                .unwrap()
                .len()
        );
        assert_eq!(
            "out/lies.wasm-files",
            parcel_conditions(&invoice, "out/lies.wasm")
                .requires
                .as_ref()
                .unwrap()[0]
        );
    }

    #[test]
    fn test_if_no_files_key_then_no_asset_parcels() {
        let invoice = expand_test_invoice("app2").unwrap();
        let count = invoice
            .parcel
            .unwrap()
            .iter()
            .filter(|parcel| parcel.member_of("wasm/no-assets.wasm-files"))
            .count();
        assert_eq!(0, count);
    }

    #[test]
    fn test_if_empty_files_key_then_no_asset_parcels() {
        let invoice = expand_test_invoice("app2").unwrap();
        let count = invoice
            .parcel
            .unwrap()
            .iter()
            .filter(|parcel| parcel.member_of("wasm/empty-assets.wasm-files"))
            .count();
        assert_eq!(0, count);
    }

    #[test]
    fn test_if_no_files_match_then_no_asset_parcels() {
        let invoice = expand_test_invoice("app2").unwrap();
        let count = invoice
            .parcel
            .unwrap()
            .iter()
            .filter(|parcel| parcel.member_of("wasm/no-match.wasm-files"))
            .count();
        assert_eq!(0, count);
    }

    #[test]
    fn test_if_nonexistent_directory_then_no_asset_parcels() {
        let invoice = expand_test_invoice("app2").unwrap();
        let count = invoice
            .parcel
            .unwrap()
            .iter()
            .filter(|parcel| parcel.member_of("wasm/no-directory.wasm-files"))
            .count();
        assert_eq!(0, count);
    }

    #[test]
    fn test_if_file_does_not_exist_then_no_asset_parcels() {
        // TODO: I feel like this should be an error
        let invoice = expand_test_invoice("app2").unwrap();
        let count = invoice
            .parcel
            .unwrap()
            .iter()
            .filter(|parcel| parcel.member_of("wasm/specific-file-missing.wasm-files"))
            .count();
        assert_eq!(0, count); // TODO: ?
    }

    #[test]
    fn test_if_handler_appears_as_an_asset_then_there_are_two_parcels_with_appropriate_membership_and_requirements_clauses(
    ) {
        let invoice = expand_test_invoice("app2").unwrap();
        let parcels = invoice.parcel.as_ref().unwrap();
        let count = parcels
            .iter()
            .filter(|parcel| parcel.member_of("wasm/other-wasms.wasm-files"))
            .count();
        assert_eq!(3, count);
        let file_occurrences = parcels
            .iter()
            .filter(|parcel| parcel.label.name == "wasm/no-assets.wasm")
            .collect::<Vec<_>>();
        assert_eq!(2, file_occurrences.len());
        let handler_parcel = file_occurrences
            .iter()
            .filter(|parcel| parcel.conditions.as_ref().unwrap().requires.is_some());
        assert_eq!(1, handler_parcel.count());
        let asset_parcel = file_occurrences
            .iter()
            .filter(|parcel| parcel.conditions.as_ref().unwrap().member_of.is_some());
        assert_eq!(1, asset_parcel.count());
    }
}
