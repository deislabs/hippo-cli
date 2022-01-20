use hippo_openapi::apis::configuration::ApiKey;
use hippo_openapi::apis::{
    account_api::api_account_createtoken_post,
    configuration::Configuration,
    revision_api::api_revision_post
};
use hippo_openapi::models::{ CreateTokenCommand, RegisterRevisionCommand };

use reqwest::header;

const JSON_MIME_TYPE: &str = "application/json";

pub struct ConnectionInfo {
    pub url: String,
    pub danger_accept_invalid_certs: bool,
    pub username: String,
    pub password: String,
}

pub async fn register(bindle_id: &bindle::Id, conn_info: &ConnectionInfo) -> anyhow::Result<()> {
    
    let mut headers = header::HeaderMap::new();
    headers.insert(header::ACCEPT, JSON_MIME_TYPE.parse().unwrap());
    headers.insert(header::CONTENT_TYPE, JSON_MIME_TYPE.parse().unwrap());

    let mut configuration = Configuration {
        base_path: conn_info.url.clone(),
        user_agent: Some(format!("{}/{}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"))),
        client: reqwest::Client::builder().danger_accept_invalid_certs(conn_info.danger_accept_invalid_certs).default_headers(headers).build()?,
        basic_auth: None,
        oauth_access_token: None,
        bearer_access_token: None,
        api_key: None,
    };
    
    let token_info = api_account_createtoken_post(
        &configuration,
        Some(CreateTokenCommand {
            user_name: Some(conn_info.username.clone()),
            password: Some(conn_info.password.clone())
        })
    ).await?;

    configuration.api_key = Some(ApiKey {
        prefix: Some("Bearer".to_owned()),
        key: token_info.token.unwrap(),
    });

    api_revision_post(
        &configuration,
        Some(RegisterRevisionCommand {
            app_storage_id: Some(bindle_id.name().to_owned()),
            revision_number: Some(bindle_id.version_string())
        })
    ).await?;

    Ok(())
}
