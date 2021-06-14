pub async fn register(bindle_id: &bindle::Id, hippo_url: &str) -> anyhow::Result<()> {
    // TODO: username and password
    let hippo_client = hippo::Client::new_from_login(hippo_url, "admin", "Passw0rd!").await?;
    hippo_client
        .register_revision_by_storage_id(bindle_id.name(), &bindle_id.version_string())
        .await?;
    Ok(())
}
