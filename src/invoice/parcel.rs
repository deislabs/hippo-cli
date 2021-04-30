//! Definition and implementation of the `Parcel` type

use serde::{Deserialize, Serialize};

use crate::invoice::{Condition, Label};

/// A description of a stored parcel file
///
/// A parcel file can be an arbitrary "blob" of data. This could be binary or text files. This
/// object contains the metadata and associated conditions for using a parcel. For more information,
/// see the [Bindle Spec](https://github.com/deislabs/bindle/blob/master/docs/bindle-spec.md)
#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq, Hash)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Parcel {
    pub label: Label,
    pub conditions: Option<Condition>,
}
