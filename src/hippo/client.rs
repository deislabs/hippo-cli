use hippo_openapi::apis::account_api::{api_account_createtoken_post, api_account_post};
use hippo_openapi::apis::app_api::{api_app_id_delete, api_app_post};
use hippo_openapi::apis::certificate_api::{api_certificate_id_delete, api_certificate_post};
use hippo_openapi::apis::channel_api::{api_channel_id_delete, api_channel_post};
use hippo_openapi::apis::configuration::{ApiKey, Configuration};
use hippo_openapi::apis::environment_variable_api::{
    api_environmentvariable_id_delete, api_environmentvariable_post,
};
use hippo_openapi::apis::revision_api::api_revision_post;
use hippo_openapi::models::{
    ChannelRevisionSelectionStrategy, CreateAccountCommand, CreateAppCommand,
    CreateCertificateCommand, CreateChannelCommand, CreateEnvironmentVariableCommand,
    CreateTokenCommand, RegisterRevisionCommand, TokenInfo,
};

use reqwest::header;

const JSON_MIME_TYPE: &str = "application/json";

pub struct ConnectionInfo {
    pub url: String,
    pub danger_accept_invalid_certs: bool,
    pub api_key: Option<String>,
}

pub struct Client {
    configuration: Configuration,
}

impl Client {
    pub fn new(conn_info: ConnectionInfo) -> Self {
        let mut headers = header::HeaderMap::new();
        headers.insert(header::ACCEPT, JSON_MIME_TYPE.parse().unwrap());
        headers.insert(header::CONTENT_TYPE, JSON_MIME_TYPE.parse().unwrap());

        let configuration = Configuration {
            base_path: conn_info.url.clone(),
            user_agent: Some(format!(
                "{}/{}",
                env!("CARGO_PKG_NAME"),
                env!("CARGO_PKG_VERSION")
            )),
            client: reqwest::Client::builder()
                .danger_accept_invalid_certs(conn_info.danger_accept_invalid_certs)
                .default_headers(headers)
                .build()
                .unwrap(),
            basic_auth: None,
            oauth_access_token: None,
            bearer_access_token: None,
            api_key: conn_info.api_key.map_or(None, |t| {
                Some(ApiKey {
                    prefix: Some("Bearer".to_owned()),
                    key: t,
                })
            }),
        };

        Self { configuration }
    }

    pub async fn register(&self, username: String, password: String) -> anyhow::Result<String> {
        let id = api_account_post(
            &self.configuration,
            Some(CreateAccountCommand {
                user_name: Some(username),
                password: Some(password.clone()),
                password_confirm: Some(password),
            }),
        )
        .await?;

        Ok(id)
    }

    pub async fn login(&self, username: String, password: String) -> anyhow::Result<TokenInfo> {
        let token = api_account_createtoken_post(
            &self.configuration,
            Some(CreateTokenCommand {
                user_name: Some(username),
                password: Some(password),
            }),
        )
        .await?;

        Ok(token)
    }

    pub async fn add_app(&self, name: String, storage_id: String) -> anyhow::Result<String> {
        let id = api_app_post(
            &self.configuration,
            Some(CreateAppCommand {
                name: Some(name),
                storage_id: Some(storage_id),
            }),
        )
        .await?;

        Ok(id)
    }

    pub async fn remove_app(&self, id: String) -> anyhow::Result<()> {
        api_app_id_delete(&self.configuration, &id).await?;
        Ok(())
    }

    pub async fn add_certificate(
        &self,
        name: String,
        public_key: String,
        private_key: String,
    ) -> anyhow::Result<String> {
        let id = api_certificate_post(
            &self.configuration,
            Some(CreateCertificateCommand {
                name: Some(name),
                public_key: Some(public_key),
                private_key: Some(private_key),
            }),
        )
        .await?;

        Ok(id)
    }

    pub async fn remove_certificate(&self, id: String) -> anyhow::Result<()> {
        api_certificate_id_delete(&self.configuration, &id).await?;

        Ok(())
    }

    pub async fn add_channel(
        &self,
        app_id: String,
        name: String,
        domain: Option<String>,
        revision_selection_strategy: Option<ChannelRevisionSelectionStrategy>,
        range_rule: Option<String>,
        active_revision_id: Option<String>,
        certificate_id: Option<String>,
    ) -> anyhow::Result<String> {
        let id = api_channel_post(
            &self.configuration,
            Some(CreateChannelCommand {
                app_id: Some(app_id),
                name: Some(name),
                domain,
                revision_selection_strategy,
                range_rule,
                active_revision_id,
                certificate_id,
            }),
        )
        .await?;

        Ok(id)
    }

    pub async fn remove_channel(&self, id: String) -> anyhow::Result<()> {
        api_channel_id_delete(&self.configuration, &id).await?;

        Ok(())
    }

    pub async fn add_environment_variable(
        &self,
        key: String,
        value: String,
        channel_id: String,
    ) -> anyhow::Result<String> {
        let id = api_environmentvariable_post(
            &self.configuration,
            Some(CreateEnvironmentVariableCommand {
                key: Some(key),
                value: Some(value),
                channel_id: Some(channel_id),
            }),
        )
        .await?;

        Ok(id)
    }

    pub async fn remove_environment_variable(&self, id: String) -> anyhow::Result<()> {
        api_environmentvariable_id_delete(&self.configuration, &id).await?;

        Ok(())
    }

    pub async fn add_revision(
        &self,
        app_storage_id: String,
        revision_number: String,
    ) -> anyhow::Result<()> {
        api_revision_post(
            &self.configuration,
            Some(RegisterRevisionCommand {
                app_storage_id: Some(app_storage_id),
                revision_number: Some(revision_number),
            }),
        )
        .await?;

        Ok(())
    }
}
