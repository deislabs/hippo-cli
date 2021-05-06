//! Contains the main invoice object definition, its implementation, and all related subobject (such
//! as `Parcel`s and `Label`s)
//!
//! ***NOTE:*** Ideally this folder should be replaced with a reference the 'bindle' crate but
//! that doesn't work on WSL at the moment.  (TODO: might be able to get rid of now?)

mod bindle_spec;
mod condition;
mod group;
mod label;
mod parcel;

#[doc(inline)]
pub use bindle_spec::BindleSpec;
#[doc(inline)]
pub use condition::Condition;
#[doc(inline)]
pub use group::Group;
#[doc(inline)]
pub use label::Label;
#[doc(inline)]
pub use parcel::Parcel;

use serde::{Deserialize, Serialize};

use std::{collections::BTreeMap, convert::TryFrom};

/// Alias for feature map in an Invoice's parcel
pub type FeatureMap = BTreeMap<String, BTreeMap<String, String>>;

/// Alias for annotations map
pub type AnnotationMap = BTreeMap<String, String>;

/// The main structure for a Bindle invoice.
///
/// The invoice describes a specific version of a bindle. For example, the bindle
/// `foo/bar/1.0.0` would be represented as an Invoice with the `BindleSpec` name
/// set to `foo/bar` and version set to `1.0.0`.
///
/// Most fields on this struct are singular to best represent the specification. There,
/// fields like `group` and `parcel` are singular due to the conventions of TOML.
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Invoice {
    pub bindle_version: String,
    pub yanked: Option<bool>,
    pub bindle: BindleSpec,
    pub annotations: Option<AnnotationMap>,
    pub parcel: Option<Vec<Parcel>>,
    pub group: Option<Vec<Group>>,
}

impl Invoice {
    pub fn id(&self) -> anyhow::Result<bindle::Id> {
        let id = bindle::Id::try_from(format!("{}/{}", &self.bindle.name, &self.bindle.version))?;
        Ok(id)
    }
}
