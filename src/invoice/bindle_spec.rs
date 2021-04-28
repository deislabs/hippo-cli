//! The specification for a bindle

use serde::{Deserialize, Serialize};

/// The specification for a bindle, that uniquely identifies the Bindle and provides additional
/// optional metadata
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct BindleSpec {
    pub name: String,
    pub version: String,
    pub description: Option<String>,
    pub authors: Option<Vec<String>>,
}
