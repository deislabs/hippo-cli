pub struct ConnectionInfo {
    pub url: String,
    pub danger_accept_invalid_certs: bool,
    pub username: String,
    pub password: String,
}

pub async fn register(bindle_id: &bindle::Id, conn_info: &ConnectionInfo) -> anyhow::Result<()> {
    let options = hippo::ClientOptions {
        danger_accept_invalid_certs: conn_info.danger_accept_invalid_certs,
    };
    let hippo_client = hippo::Client::new_with_options(
        &conn_info.url,
        &conn_info.username,
        &conn_info.password,
        options,
    )
    .await?;
    hippo_client
        .register_revision_by_storage_id(bindle_id.name(), &bindle_id.version_string())
        .await?;
    Ok(())
}
