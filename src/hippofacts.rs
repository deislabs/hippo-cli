use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, convert::TryFrom};

// type FeatureMap = BTreeMap<String, BTreeMap<String, String>>;

type AnnotationMap = BTreeMap<String, String>;

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct RawHippoFacts {
    pub bindle: BindleSpec,
    pub annotations: Option<AnnotationMap>,
    pub handler: Option<Vec<RawHandler>>,
    pub export: Option<Vec<RawExport>>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct BindleSpec {
    pub name: String,
    pub version: String, // not semver::Version because this could be a template
    pub description: Option<String>,
    pub authors: Option<Vec<String>>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct RawHandler {
    pub name: Option<String>,
    pub external: Option<RawExternalRef>,
    pub route: String,
    pub files: Option<Vec<String>>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct RawExport {
    pub name: String,
    pub id: String,
    pub files: Option<Vec<String>>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct RawExternalRef {
    pub bindle_id: String,
    pub handler_id: String,
}

pub struct HippoFacts {
    pub bindle: BindleSpec,
    pub annotations: Option<AnnotationMap>,
    pub entries: Vec<HippoFactsEntry>,
}

pub struct LocalHandler {
    pub name: String,
    pub route: String,
    pub files: Option<Vec<String>>,
}

pub struct ExternalHandler {
    pub external: ExternalRef,
    pub route: String,
    pub files: Option<Vec<String>>,
}

enum HandlerModule {
    File(String),
    External(ExternalRef),
}

#[derive(Clone)]
pub struct ExternalRef {
    pub bindle_id: bindle::Id,
    pub handler_id: String,
}

pub struct Export {
    pub name: String,
    pub id: String,
    pub files: Vec<String>,
}

pub enum HippoFactsEntry {
    LocalHandler(LocalHandler),
    ExternalHandler(ExternalHandler),
    Export(Export),
}

impl HippoFacts {
    pub fn read_from(source: impl AsRef<std::path::Path>) -> anyhow::Result<Self> {
        // Immediate-call closure lets us use the try operator
        let read_result = (|| {
            let content = std::fs::read_to_string(&source)?;
            let spec = toml::from_str::<RawHippoFacts>(&content)?;
            Self::try_from(&spec)
        })();
        read_result.map_err(|e: anyhow::Error| {
            anyhow::anyhow!(
                "Error parsing {} as a Hippo artifacts file: {}",
                source.as_ref().to_string_lossy(),
                e
            )
        })
    }
}

impl TryFrom<&RawHippoFacts> for HippoFacts {
    type Error = anyhow::Error;

    fn try_from(raw: &RawHippoFacts) -> anyhow::Result<Self> {
        let handler_vec = raw.handler.clone().unwrap_or_default();
        let export_vec = raw.export.clone().unwrap_or_default();
        let entries = handler_vec
            .iter()
            .map(HippoFactsEntry::try_from)
            .chain(export_vec.iter().map(HippoFactsEntry::try_from))
            .collect::<anyhow::Result<Vec<_>>>()?;
        if entries.is_empty() {
            return Err(no_handlers());
        }
        Ok(Self {
            bindle: raw.bindle.clone(),
            annotations: raw.annotations.clone(),
            entries,
        })
    }
}

impl TryFrom<&RawHandler> for HippoFactsEntry {
    type Error = anyhow::Error;

    fn try_from(raw: &RawHandler) -> anyhow::Result<Self> {
        let handler_module = match (&raw.name, &raw.external) {
            (Some(name), None) => Ok(HandlerModule::File(name.clone())),
            (None, Some(external_ref)) => Ok(HandlerModule::External(ExternalRef::try_from(external_ref)?)),
            _ => Err(anyhow::anyhow!("Route '{}' must specify exactly one of 'name' and 'external'", raw.route)),
        }?;
        let entry = match handler_module {
            HandlerModule::File(name) => Self::LocalHandler(LocalHandler {
                name,
                route: raw.route.clone(),
                files: raw.files.clone(),
            }),
            HandlerModule::External(external) => Self::ExternalHandler(ExternalHandler {
                external,
                route: raw.route.clone(),
                files: raw.files.clone(),
            }),
        };
        Ok(entry)
    }
}

impl TryFrom<&RawExternalRef> for ExternalRef {
    type Error = anyhow::Error;

    fn try_from(raw: &RawExternalRef) -> anyhow::Result<ExternalRef> {
        let bindle_id = bindle::Id::try_from(&raw.bindle_id)?;
        Ok(Self {
            bindle_id,
            handler_id: raw.handler_id.clone(),
        })
    }
}

impl TryFrom<&RawExport> for HippoFactsEntry {
    type Error = anyhow::Error;

    fn try_from(raw: &RawExport) -> anyhow::Result<Self> {
        Ok(Self::Export(Export {
            id: raw.id.clone(),
            name: raw.name.clone(),
            files: raw.files.clone().unwrap_or_default(),
        }))
    }
}

impl HippoFactsEntry {
    pub fn files(&self) -> Vec<String> {
        match self {
            Self::LocalHandler(h) => h.files.clone().unwrap_or_default(),
            Self::ExternalHandler(h) => h.files.clone().unwrap_or_default(),
            Self::Export(e) => e.files.clone(),
        }
    }

    pub fn external_ref(&self) -> Option<ExternalRef> {
        match self {
            Self::LocalHandler(_) => None,
            Self::ExternalHandler(h) => Some(h.external.clone()),
            Self::Export(_) => None,
        }
    }
}

fn no_handlers() -> anyhow::Error {
    anyhow::anyhow!("No handlers defined in artifact spec")
}

#[cfg(test)]
mod test {
    use super::*;
    
    impl HippoFactsEntry {
        pub fn name(&self) -> Option<String> {
            match self {
                Self::LocalHandler(h) => Some(h.name.clone()),
                Self::ExternalHandler(_) => None,
                Self::Export(e) => Some(e.name.clone()),
            }
        }
    
        pub fn route(&self) -> Option<String> {
            match self {
                Self::LocalHandler(h) => Some(h.route.clone()),
                Self::ExternalHandler(h) => Some(h.route.clone()),
                Self::Export(_) => None,
            }
        }
    
        pub fn export_id(&self) -> Option<String> {
            match self {
                Self::LocalHandler(_) => None,
                Self::ExternalHandler(_) => None,
                Self::Export(e) => Some(e.id.clone()),
            }
        }
    }
    
    #[test]
    fn test_can_read_hippo_facts() {
        let raw: RawHippoFacts = toml::from_str(
            r#"
        # HIPPO FACT: the North American house hippo is found across Canada and the Eastern US
        [bindle]
        name = "birds"
        version = "1.2.4"

        [[handler]]
        name = "penguin.wasm"
        route = "/birds/flightless"
        files = ["adelie.png", "rockhopper.png", "*.jpg"]

        [[handler]]
        name = "cassowary.wasm"
        route = "/birds/savage/rending"
        "#,
        )
        .expect("error parsing test TOML");
        let facts = HippoFacts::try_from(&raw).expect("error parsing raw to HF");

        assert_eq!("birds", &facts.bindle.name);
        assert_eq!(&None, &facts.annotations);

        let handlers = &facts.entries;

        assert_eq!(2, handlers.len());

        assert_eq!("penguin.wasm", handlers[0].name().unwrap());
        assert_eq!("/birds/flightless", &handlers[0].route().unwrap());
        assert_eq!(3, handlers[0].files().len());

        assert_eq!("cassowary.wasm", handlers[1].name().unwrap());
        assert_eq!("/birds/savage/rending", &handlers[1].route().unwrap());
        assert_eq!(0, handlers[1].files().len());
    }

    #[test]
    fn test_parse_externals() {
        let facts = HippoFacts::read_from("./testdata/external1/HIPPOFACTS")
            .expect("error reading facts file");

        assert_eq!("toastbattle", &facts.bindle.name);

        let handlers = &facts.entries;

        assert_eq!(2, handlers.len());

        let ext = handlers[1].external_ref().unwrap();
        assert_eq!("deislabs/fileserver", ext.bindle_id.name());
        assert_eq!("1.0.3", ext.bindle_id.version_string());
        assert_eq!("static", ext.handler_id);
    }

    #[test]
    fn test_parse_exports() {
        let facts =
            HippoFacts::read_from("./testdata/lib1/HIPPOFACTS").expect("error reading facts file");

        assert_eq!("server", &facts.bindle.name);

        assert_eq!(2, facts.entries.len());

        let server_export = &facts.entries[0];
        assert_eq!("wasm/server.wasm", server_export.name().unwrap());
        assert_eq!("serve_all_the_things", server_export.export_id().unwrap());
        assert_eq!(0, server_export.files().len());
    }

    #[test]
    fn test_no_handlers_no_exports_no_service() {
        let raw: RawHippoFacts = toml::from_str(
            r#"
        # HIPPO FACT: Hippos are the second heaviest land animal after the elephant
        [bindle]
        name = "nope"
        version = "1.2.4"
        "#,
        )
        .expect("error parsing test TOML");
        let facts = HippoFacts::try_from(&raw);

        assert!(facts.is_err());
        if let Err(e) = facts {
            assert!(
                e.to_string().contains("No handlers"),
                "check error message is helpful: '{}'",
                e
            );
        }
    }
}
