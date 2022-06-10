use std::collections::HashMap;

use hippo_openapi::apis::account_api::{api_account_createtoken_post, api_account_post};
use hippo_openapi::apis::app_api::{api_app_get, api_app_id_delete, api_app_post};
use hippo_openapi::apis::certificate_api::{
    api_certificate_get, api_certificate_id_delete, api_certificate_post,
};
use hippo_openapi::apis::channel_api::{
    api_channel_channel_id_get, api_channel_get, api_channel_id_delete, api_channel_post,
};
use hippo_openapi::apis::configuration::{ApiKey, Configuration};
use hippo_openapi::apis::environment_variable_api::{
    api_environmentvariable_get, api_environmentvariable_id_delete, api_environmentvariable_post,
};
use hippo_openapi::apis::revision_api::{api_revision_get, api_revision_post};
use hippo_openapi::apis::Error;
use hippo_openapi::models::{
    AppsVm, CertificatesVm, ChannelDto, ChannelRevisionSelectionStrategy, ChannelsVm,
    CreateAccountCommand, CreateAppCommand, CreateCertificateCommand, CreateChannelCommand,
    CreateEnvironmentVariableCommand, CreateTokenCommand, EnvironmentVariablesVm,
    RegisterRevisionCommand, RevisionsVm, TokenInfo,
};

use reqwest::header;
use serde::Deserialize;

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
        api_account_post(
            &self.configuration,
            Some(CreateAccountCommand {
                user_name: username,
                password: password,
            }),
        )
        .await
        .map_err(format_response_error)
    }

    pub async fn login(&self, username: String, password: String) -> anyhow::Result<TokenInfo> {
        api_account_createtoken_post(
            &self.configuration,
            Some(CreateTokenCommand {
                user_name: username,
                password: password,
            }),
        )
        .await
        .map_err(format_response_error)
    }

    pub async fn add_app(&self, name: String, storage_id: String) -> anyhow::Result<String> {
        api_app_post(
            &self.configuration,
            Some(CreateAppCommand {
                name: name,
                storage_id: storage_id,
            }),
        )
        .await
        .map_err(format_response_error)
    }

    pub async fn remove_app(&self, id: String) -> anyhow::Result<()> {
        api_app_id_delete(&self.configuration, &id)
            .await
            .map_err(format_response_error)
    }

    pub async fn list_apps(&self) -> anyhow::Result<AppsVm> {
        api_app_get(&self.configuration)
            .await
            .map_err(format_response_error)
    }

    pub async fn add_certificate(
        &self,
        name: String,
        public_key: String,
        private_key: String,
    ) -> anyhow::Result<String> {
        api_certificate_post(
            &self.configuration,
            Some(CreateCertificateCommand {
                name: name,
                public_key: public_key,
                private_key: private_key,
            }),
        )
        .await
        .map_err(format_response_error)
    }

    pub async fn list_certificates(&self) -> anyhow::Result<CertificatesVm> {
        api_certificate_get(&self.configuration)
            .await
            .map_err(format_response_error)
    }

    pub async fn remove_certificate(&self, id: String) -> anyhow::Result<()> {
        api_certificate_id_delete(&self.configuration, &id)
            .await
            .map_err(format_response_error)
    }

    pub async fn add_channel(
        &self,
        app_id: String,
        name: String,
        domain: Option<String>,
        revision_selection_strategy: ChannelRevisionSelectionStrategy,
        range_rule: Option<String>,
        active_revision_id: Option<String>,
        certificate_id: Option<String>,
    ) -> anyhow::Result<String> {
        let command = CreateChannelCommand {
            app_id: app_id,
            name: name,
            domain,
            revision_selection_strategy,
            range_rule,
            active_revision_id,
            certificate_id,
        };
        api_channel_post(&self.configuration, Some(command))
            .await
            .map_err(format_response_error)
    }

    #[allow(dead_code)]
    pub async fn get_channel_by_id(&self, id: &str) -> anyhow::Result<ChannelDto> {
        api_channel_channel_id_get(&self.configuration, id)
            .await
            .map_err(format_response_error)
    }

    pub async fn list_channels(&self) -> anyhow::Result<ChannelsVm> {
        api_channel_get(&self.configuration)
            .await
            .map_err(format_response_error)
    }

    pub async fn remove_channel(&self, id: String) -> anyhow::Result<()> {
        api_channel_id_delete(&self.configuration, &id)
            .await
            .map_err(format_response_error)
    }

    pub async fn add_environment_variable(
        &self,
        key: String,
        value: String,
        channel_id: String,
    ) -> anyhow::Result<String> {
        api_environmentvariable_post(
            &self.configuration,
            Some(CreateEnvironmentVariableCommand {
                key: key,
                value: value,
                channel_id: channel_id,
            }),
        )
        .await
        .map_err(format_response_error)
    }

    pub async fn list_environmentvariables(&self) -> anyhow::Result<EnvironmentVariablesVm> {
        api_environmentvariable_get(&self.configuration)
            .await
            .map_err(format_response_error)
    }

    pub async fn remove_environment_variable(&self, id: String) -> anyhow::Result<()> {
        api_environmentvariable_id_delete(&self.configuration, &id)
            .await
            .map_err(format_response_error)
    }

    pub async fn add_revision(
        &self,
        app_storage_id: String,
        revision_number: String,
    ) -> anyhow::Result<()> {
        api_revision_post(
            &self.configuration,
            Some(RegisterRevisionCommand {
                app_storage_id: app_storage_id,
                revision_number: revision_number,
            }),
        )
        .await
        .map_err(format_response_error)
    }

    pub async fn list_revisions(&self) -> anyhow::Result<RevisionsVm> {
        api_revision_get(&self.configuration)
            .await
            .map_err(format_response_error)
    }
}

#[derive(Deserialize, Debug)]
struct ValidationExceptionMessage {
    title: String,
    errors: HashMap<String, Vec<String>>,
}

fn format_response_error<T>(e: Error<T>) -> anyhow::Error {
    match e {
        Error::ResponseError(r) => {
            match serde_json::from_str::<ValidationExceptionMessage>(&r.content) {
                Ok(m) => anyhow::anyhow!("{} {:?}", m.title, m.errors),
                _ => anyhow::anyhow!(r.content),
            }
        }
        _ => anyhow::anyhow!(e.to_string()),
    }
}
