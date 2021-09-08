use std::path::Path;

use crate::bindle_utils::BindleConnectionInfo;

pub async fn push_all(
    path: impl AsRef<Path>,
    bindle_id: &bindle::Id,
    bindle_connection: &BindleConnectionInfo,
) -> anyhow::Result<()> {
    bindle_connection.push_all(path, bindle_id).await
}
