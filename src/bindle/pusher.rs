use std::path::Path;

use bindle::standalone::StandaloneRead;

use super::client::ConnectionInfo;

pub async fn push_all(
    path: impl AsRef<Path>,
    bindle_id: &bindle::Id,
    bindle_connection: &ConnectionInfo,
) -> anyhow::Result<()> {
    let reader = StandaloneRead::new(&path, bindle_id).await?;
    let client = bindle_connection.client()?;
    reader
        .push(&client)
        .await
        .map_err(|e| anyhow::anyhow!("Error pushing bindle to server: {}", e))?;
    Ok(())
}
