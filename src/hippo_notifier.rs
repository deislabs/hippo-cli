pub struct ConnectionInfo {
    pub url: String,
    pub username: String,
    pub password: String,
}

pub async fn register(bindle_id: &bindle::Id, conn_info: &ConnectionInfo) -> anyhow::Result<()> {
    // TODO: username and password
    let hippo_client = hippo::Client::new_from_login(&conn_info.url, &conn_info.username, &conn_info.password).await?;
    hippo_client
        .register_revision_by_storage_id(bindle_id.name(), &bindle_id.version_string())
        .await?;
    Ok(())
}
