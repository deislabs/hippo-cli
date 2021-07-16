use std::collections::{BTreeMap, HashMap, HashSet};
use std::convert::TryFrom;
use std::iter::FromIterator;
use std::path::{Path, PathBuf};

use bindle::{AnnotationMap, BindleSpec, Condition, Group, Invoice, Label, Parcel};
use glob::GlobError;
use itertools::Itertools;
use sha2::{Digest, Sha256};

use crate::bindle_utils::InvoiceHelpers;
use crate::hippofacts::{ExternalRef, HippoFacts, HippoFactsEntry};

pub struct ExpansionContext {
    pub relative_to: PathBuf,
    pub invoice_versioning: InvoiceVersioning,
    pub external_invoices: HashMap<bindle::Id, Invoice>,
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
    let groups = expand_all_entries_to_groups(&hippofacts)?;
    let handler_parcels = expand_module_entries_to_parcels(&hippofacts, expansion_context)?;
    let external_dependent_parcels =
        expand_all_external_ref_dependencies_to_parcels(&hippofacts, expansion_context)?;
    let file_parcels = expand_all_files_to_parcels(&hippofacts, expansion_context)?;
    check_for_name_clashes(&external_dependent_parcels, &file_parcels)?;
    let parcels = handler_parcels
        .into_iter()
        .chain(external_dependent_parcels)
        .chain(file_parcels)
        .collect();

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

fn expand_all_entries_to_groups(hippofacts: &HippoFacts) -> anyhow::Result<Vec<Group>> {
    let groups = hippofacts.entries.iter().map(expand_to_group).collect();
    Ok(groups)
}

fn expand_to_group(entry: &HippoFactsEntry) -> Group {
    Group {
        name: group_name(entry),
        required: None,
        satisfied_by: None,
    }
}

fn group_name(entry: &HippoFactsEntry) -> String {
    match entry {
        HippoFactsEntry::LocalHandler(h) => format!("{}-files", h.name),
        HippoFactsEntry::ExternalHandler(h) => format!(
            "import:{}:{}-at-{}-files",
            &h.external.bindle_id, &h.external.handler_id, &h.route
        ),
        HippoFactsEntry::Export(e) => format!("{}-{}-files", e.name, e.id),
    }
}

fn expand_module_entries_to_parcels(
    hippofacts: &HippoFacts,
    expansion_context: &ExpansionContext,
) -> anyhow::Result<Vec<Parcel>> {
    let parcels = hippofacts
        .entries
        .iter()
        .map(|handler| expand_one_module_entry_to_parcel(handler, expansion_context));
    parcels.collect()
}

fn expand_one_module_entry_to_parcel(
    entry: &HippoFactsEntry,
    expansion_context: &ExpansionContext,
) -> anyhow::Result<Parcel> {
    match &entry {
        HippoFactsEntry::LocalHandler(h) => convert_one_match_to_parcel(
            PathBuf::from(expansion_context.to_absolute(&h.name)),
            expansion_context,
            vec![("route", &h.route), ("file", "false")],
            None,
            None,
            Some(&group_name(entry)),
        ),
        HippoFactsEntry::ExternalHandler(e) => convert_one_ref_to_parcel(
            &e.external,
            expansion_context,
            vec![("route", &e.route), ("file", "false")],
            None,
            Some(&group_name(entry)),
        ),
        HippoFactsEntry::Export(e) => convert_one_match_to_parcel(
            PathBuf::from(expansion_context.to_absolute(&e.name)),
            expansion_context,
            vec![("file", "false")],
            Some(vec![("wagi_handler_id", &e.id)]),
            None,
            Some(&group_name(entry)),
        ),
    }
}

fn expand_all_external_ref_dependencies_to_parcels(
    hippofacts: &HippoFacts,
    expansion_context: &ExpansionContext,
) -> anyhow::Result<Vec<Parcel>> {
    let parcel_lists = hippofacts.entries.iter().map(|handler| match &handler {
        HippoFactsEntry::ExternalHandler(e) => expand_one_external_ref_dependencies_to_parcels(
            &e.external,
            expansion_context,
            &group_name(handler),
        ),
        _ => Ok(vec![]),
    });
    let parcels = flatten_or_fail(parcel_lists)?;
    Ok(merge_memberships(parcels))
}

fn expand_one_external_ref_dependencies_to_parcels(
    external_ref: &ExternalRef,
    expansion_context: &ExpansionContext,
    dest_group_name: &str,
) -> anyhow::Result<Vec<Parcel>> {
    let parcels = (|| {
        let invoice = expansion_context
            .external_invoices
            .get(&external_ref.bindle_id)
            .ok_or_else(|| anyhow::anyhow!("external invoice not found on server"))?;
        let main_parcel = find_handler_parcel(invoice, &external_ref.handler_id)
            .ok_or_else(|| anyhow::anyhow!("external invoice does not contain specified parcel"))?;
        let required_parcels = invoice.parcels_required_by(&main_parcel);
        let parcel_copies = required_parcels.iter().map(|p| Parcel {
            label: Label {
                annotations: annotation_do_not_stage_file(),
                ..p.label.clone()
            },
            conditions: Some(Condition {
                member_of: Some(vec![dest_group_name.to_owned()]),
                requires: None,
            }),
        });
        Ok(parcel_copies.collect())
    })();
    parcels.map_err(|e: anyhow::Error| {
        anyhow::anyhow!(
            "Could not copy dependency tree for external ref {}:{}: {}",
            external_ref.bindle_id,
            external_ref.handler_id,
            e
        )
    })
}

fn expand_all_files_to_parcels(
    hippofacts: &HippoFacts,
    expansion_context: &ExpansionContext,
) -> anyhow::Result<Vec<Parcel>> {
    let parcel_lists = hippofacts
        .entries
        .iter()
        .map(|handler| expand_files_to_parcels(handler, expansion_context));
    let parcels = flatten_or_fail(parcel_lists)?;
    Ok(merge_memberships(parcels))
}

fn expand_files_to_parcels(
    entry: &HippoFactsEntry,
    expansion_context: &ExpansionContext,
) -> anyhow::Result<Vec<Parcel>> {
    let patterns = entry.files();
    let parcels = patterns
        .iter()
        .map(|f| expand_file_to_parcels(f, expansion_context, &group_name(entry)));
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
        Err(e) => Err(anyhow::anyhow!("Couldn't expand pattern: {}", e)),
        Ok(path) => {
            let features = vec![("file", "true")];
            convert_one_match_to_parcel(
                path,
                expansion_context,
                features,
                None,
                Some(member_of),
                None,
            )
        }
    }
}

fn convert_one_match_to_parcel(
    path: PathBuf,
    expansion_context: &ExpansionContext,
    wagi_features: Vec<(&str, &str)>,
    wagi_annotations: Option<Vec<(&str, &str)>>,
    member_of: Option<&str>,
    requires: Option<&str>,
) -> anyhow::Result<Parcel> {
    // Immediate-call closure allows us to use the try operator
    let parcel = (|| {
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

        let annotations = wagi_annotations.map(map_of);
        let feature = Some(wagi_feature_of(wagi_features));

        Ok(Parcel {
            label: Label {
                name,
                sha256: digest_string,
                media_type,
                size,
                feature,
                annotations,
            },
            conditions: Some(Condition {
                member_of: vector_of(member_of),
                requires: vector_of(requires),
            }),
        })
    })();
    parcel.map_err(|e: anyhow::Error| {
        anyhow::anyhow!(
            "Could not assemble parcel for file {}: {}",
            path.to_string_lossy(),
            e
        )
    })
}

fn convert_one_ref_to_parcel(
    external_ref: &ExternalRef,
    expansion_context: &ExpansionContext,
    wagi_features: Vec<(&str, &str)>,
    member_of: Option<&str>,
    requires: Option<&str>,
) -> anyhow::Result<Parcel> {
    // Immediate-call closure allows us to use the try operator
    let parcel = (|| {
        // We don't need to give the IDs in these messages because these will be prepended when
        // mapping errors that escape the closure (the parcel.map_err below)
        let invoice = expansion_context
            .external_invoices
            .get(&external_ref.bindle_id)
            .ok_or_else(|| anyhow::anyhow!("external invoice not found on server"))?;
        let parcel = find_handler_parcel(invoice, &external_ref.handler_id)
            .ok_or_else(|| anyhow::anyhow!("external invoice does not contain specified parcel"))?;

        let feature = Some(wagi_feature_of(wagi_features));

        Ok(Parcel {
            label: Label {
                name: parcel.label.name.clone(),
                sha256: parcel.label.sha256.clone(),
                media_type: parcel.label.media_type.clone(),
                size: parcel.label.size,
                annotations: annotation_do_not_stage_file(),
                feature,
            },
            conditions: Some(Condition {
                member_of: vector_of(member_of),
                requires: vector_of(requires),
            }),
        })
    })();
    parcel.map_err(|e: anyhow::Error| {
        anyhow::anyhow!(
            "Could not assemble parcel for external ref {}:{}: {}",
            external_ref.bindle_id,
            external_ref.handler_id,
            e
        )
    })
}

fn find_handler_parcel<'a>(invoice: &'a Invoice, handler_id: &'a str) -> Option<&'a Parcel> {
    match invoice.parcel.as_ref() {
        None => None,
        Some(parcels) => parcels.iter().find(|p| has_handler_id(p, handler_id)),
    }
}

fn has_handler_id(parcel: &Parcel, handler_id: &str) -> bool {
    match parcel.label.annotations.as_ref() {
        None => false,
        Some(map) => map.get("wagi_handler_id") == Some(&handler_id.to_owned()),
    }
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

fn check_for_name_clashes(
    external_dependent_parcels: &Vec<Parcel>,
    file_parcels: &Vec<Parcel>,
) -> anyhow::Result<()> {
    let file_parcel_names: HashSet<_> = file_parcels
        .iter()
        .map(|p| p.label.name.to_owned())
        .collect();
    for parcel in external_dependent_parcels {
        if file_parcel_names.contains(&parcel.label.name) {
            return Err(anyhow::anyhow!(
                "{} occurs both as a local file and as a dependency of an external reference",
                parcel.label.name
            ));
        }
    }
    Ok(())
}

fn current_user() -> Option<String> {
    std::env::var("USER")
        .or_else(|_| std::env::var("USERNAME"))
        .ok()
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
    BTreeMap::from_iter(vec![("wagi".to_owned(), map_of(values))])
}

fn map_of(values: Vec<(&str, &str)>) -> BTreeMap<String, String> {
    values
        .into_iter()
        .map(|(k, v)| (k.to_owned(), v.to_owned()))
        .collect()
}

fn annotation_do_not_stage_file() -> Option<AnnotationMap> {
    let mut annotations = AnnotationMap::new();
    annotations.insert("hippofactory_do_not_stage".to_owned(), "true".to_owned());
    Some(annotations)
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
        HippoFacts::read_from(path)
    }

    fn parcel_named<'a>(invoice: &'a Invoice, parcel_name: &str) -> &'a Parcel {
        invoice
            .parcel
            .as_ref()
            .unwrap()
            .iter()
            .find(|p| p.label.name == parcel_name)
            .expect(&format!("No parcel named {}", parcel_name))
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

    fn parcel_memberships<'a>(invoice: &'a Invoice, parcel_name: &str) -> &'a Vec<String> {
        parcel_conditions(invoice, parcel_name)
            .member_of
            .as_ref()
            .unwrap()
    }

    fn parcel_requirements<'a>(invoice: &'a Invoice, parcel_name: &str) -> &'a Vec<String> {
        parcel_conditions(invoice, parcel_name)
            .requires
            .as_ref()
            .unwrap()
    }

    fn expand_test_invoice(name: &str) -> anyhow::Result<Invoice> {
        let dir = test_dir(name);
        let hippofacts = read_hippofacts(dir.join("HIPPOFACTS")).unwrap();
        let expansion_context = ExpansionContext {
            relative_to: dir,
            invoice_versioning: InvoiceVersioning::Production,
            external_invoices: external_test_invoices(),
        };
        expand(&hippofacts, &expansion_context)
    }

    fn external_test_invoices() -> HashMap<bindle::Id, Invoice> {
        let mut invoices = HashMap::new();

        let fs_id = bindle::Id::from_str("deislabs/fileserver/1.0.3").unwrap();
        let fs_parcels = vec![
            Parcel {
                label: Label {
                    name: "experimental_file_server.gr.wasm".to_owned(),
                    sha256: "987654".to_owned(),
                    media_type: "application/wasm".to_owned(),
                    size: 123,
                    annotations: None,
                    feature: None,
                },
                conditions: None,
            },
            Parcel {
                label: Label {
                    name: "file_server.gr.wasm".to_owned(),
                    sha256: "123456789".to_owned(),
                    media_type: "application/wasm".to_owned(),
                    size: 100,
                    annotations: Some(
                        vec![("wagi_handler_id".to_owned(), "static".to_owned())]
                            .into_iter()
                            .collect(),
                    ),
                    feature: None,
                },
                conditions: None,
            },
            Parcel {
                label: Label {
                    name: "imagegallery.wasm".to_owned(),
                    sha256: "13463".to_owned(),
                    media_type: "application/wasm".to_owned(),
                    size: 234,
                    annotations: Some(
                        vec![("wagi_handler_id".to_owned(), "image_gallery".to_owned())]
                            .into_iter()
                            .collect(),
                    ),
                    feature: None,
                },
                conditions: Some(Condition {
                    member_of: None,
                    requires: Some(vec!["igfiles".to_owned()]),
                }),
            },
            Parcel {
                label: Label {
                    name: "images.db".to_owned(),
                    sha256: "134632".to_owned(),
                    media_type: "application/octet-stream".to_owned(),
                    size: 345,
                    annotations: None,
                    feature: None,
                },
                conditions: Some(Condition {
                    member_of: Some(vec!["igfiles".to_owned()]),
                    requires: None,
                }),
            },
            Parcel {
                label: Label {
                    name: "thumbnails.db".to_owned(),
                    sha256: "444444".to_owned(),
                    media_type: "application/octet-stream".to_owned(),
                    size: 456,
                    annotations: None,
                    feature: None,
                },
                conditions: Some(Condition {
                    member_of: Some(vec!["igfiles".to_owned()]),
                    requires: Some(vec!["thumbfiles".to_owned()]),
                }),
            },
            Parcel {
                label: Label {
                    name: "thumblywumbly.txt".to_owned(), // give me a break I am running out of ideas
                    sha256: "555555".to_owned(),
                    media_type: "text/plain".to_owned(),
                    size: 456,
                    annotations: None,
                    feature: None,
                },
                conditions: Some(Condition {
                    member_of: Some(vec!["thumbfiles".to_owned()]),
                    requires: None,
                }),
            },
            Parcel {
                label: Label {
                    name: "unused.txt".to_owned(),
                    sha256: "3948759834765".to_owned(),
                    media_type: "text/plain".to_owned(),
                    size: 456,
                    annotations: None,
                    feature: None,
                },
                conditions: Some(Condition {
                    member_of: Some(vec!["group_that_does_not_exist".to_owned()]),
                    requires: None,
                }),
            },
        ];
        let fs_invoice = Invoice {
            bindle_version: "1.0.0".to_owned(),
            yanked: None,
            bindle: bindle::BindleSpec {
                id: fs_id.clone(),
                description: None,
                authors: None,
            },
            annotations: None,
            parcel: Some(fs_parcels),
            group: None,
            signature: None,
        };
        invoices.insert(fs_id.clone(), fs_invoice);

        invoices
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

    #[test]
    fn test_externals_are_surfaced_as_parcels() {
        let invoice = expand_test_invoice("external1").unwrap();
        let parcels = invoice.parcel.as_ref().unwrap();
        let ext_parcel = parcel_named(&invoice, "file_server.gr.wasm");
        assert_eq!("123456789", ext_parcel.label.sha256);
        assert_eq!("application/wasm", ext_parcel.label.media_type);
        assert_eq!(100, ext_parcel.label.size);
        assert_eq!(5, parcels.len()); // 1 local handler, 1 ext handler, 3 asset files
    }

    #[test]
    fn test_externals_bring_along_their_dependencies() {
        let invoice = expand_test_invoice("external2").unwrap();
        let parcels = invoice.parcel.as_ref().unwrap();
        assert_eq!(8, parcels.len()); // 1 local handler, 1 ext handler, 3 asset files, 2 immediate ext deps, 1 indirect ext dep
    }

    #[test]
    fn test_externals_cannot_clash_with_local_files() {
        let invoice = expand_test_invoice("external3");
        assert!(invoice.is_err());
        if let Err(e) = invoice {
            let message = format!("{}", e);
            assert!(message.contains("thumbnails.db"));
            assert!(message.contains(
                "occurs both as a local file and as a dependency of an external reference"
            ));
        }
    }

    #[test]
    fn test_exports_have_the_wagi_handler_annotation() {
        let invoice = expand_test_invoice("lib1").unwrap();
        let parcels = invoice.parcel.as_ref().unwrap();
        assert_eq!(4, parcels.len());

        let exported_parcel = parcel_named(&invoice, "wasm/server.wasm");

        match exported_parcel.label.annotations.as_ref() {
            None => assert!(false, "No annotations on the exported parcel"),
            Some(map) => assert_eq!("serve_all_the_things", map.get("wagi_handler_id").unwrap()),
        };
    }

    #[test]
    fn test_exports_bring_along_their_dependencies() {
        let invoice = expand_test_invoice("lib1").unwrap();
        let parcels = invoice.parcel.as_ref().unwrap();
        assert_eq!(4, parcels.len());

        assert_eq!(
            "wasm/gallery.wasm-image_gallery-files",
            parcel_requirements(&invoice, "wasm/gallery.wasm")[0]
        );

        assert_eq!(
            "wasm/gallery.wasm-image_gallery-files",
            parcel_memberships(&invoice, "gallery/images.db")[0]
        );
        assert_eq!(
            "wasm/gallery.wasm-image_gallery-files",
            parcel_memberships(&invoice, "gallery/thumbnails.db")[0]
        );
    }
}
