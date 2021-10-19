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
        .await
        .map_err(format_register_revision_error)
}

fn format_register_revision_error(e: hippo::ClientError) -> anyhow::Error {
    let message = match &e {
        hippo::ClientError::InvalidRequest { status_code, message } => {
            let detail_clause = match message {
                Some(m) => format!(": error was {}", m),
                None => "".to_owned(),
            };
            match status_code {
                &reqwest::StatusCode::BAD_REQUEST =>
                    format!("Hippo couldn't understand the request{} (400 Bad Request)", detail_clause),
                &reqwest::StatusCode::UNAUTHORIZED =>
                    format!("Login failed: please check your credentials{} (401 Unauthorized)", detail_clause),
                &reqwest::StatusCode::FORBIDDEN =>
                    format!("Hippo can't register this revision{} (403 Forbidden)", detail_clause),
                &reqwest::StatusCode::NOT_FOUND =>
                    "You don't have access to any applications that use this bindle ID (404 Not Found)".to_owned(),
                &reqwest::StatusCode::METHOD_NOT_ALLOWED =>
                    // At one point this could be returned for TLS mismatch; not sure if we fixed that
                    format!("Wrong HTTP method; may also indicate you need to use/turn off HTTPS{} (405 Method Not Allowed)", detail_clause),
                &reqwest::StatusCode::CONFLICT =>
                    format!("Revision conflicts with existing Hippo state; it might already exist{} (409 Conflict)", detail_clause),
                &reqwest::StatusCode::IM_A_TEAPOT =>
                    "Specified URL is for a teapot not a Hippo (418 I'm a Teapot)".to_owned(),
                _ => e.to_string(),
            }
        },
        _ => e.to_string(),
    };
    anyhow::anyhow!("Error registering revision: {}", message)
}
