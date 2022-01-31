use std::sync::Arc;

use bindle::client::{
    tokens::{HttpBasic, NoToken, TokenManager},
    Client, ClientBuilder,
};

pub struct ConnectionInfo {
    base_url: String,
    allow_insecure: bool,
    token_manager: AnyAuth,
}

impl ConnectionInfo {
    pub fn new<I: Into<String>>(
        base_url: I,
        allow_insecure: bool,
        username: Option<String>,
        password: Option<String>,
    ) -> Self {
        let token_manager: Box<dyn TokenManager + Send + Sync> = match (username, password) {
            (Some(u), Some(p)) => Box::new(HttpBasic::new(&u, &p)),
            _ => Box::new(NoToken::default()),
        };

        Self {
            base_url: base_url.into(),
            allow_insecure,
            token_manager: AnyAuth {
                token_manager: Arc::new(token_manager),
            },
        }
    }

    pub fn client(&self) -> bindle::client::Result<Client<AnyAuth>> {
        let builder = ClientBuilder::default()
            .http2_prior_knowledge(false)
            .danger_accept_invalid_certs(self.allow_insecure);
        builder.build(&self.base_url, self.token_manager.clone())
    }
}

#[derive(Clone)]
pub struct AnyAuth {
    token_manager: Arc<Box<dyn TokenManager + Send + Sync>>,
}

#[async_trait::async_trait]
impl TokenManager for AnyAuth {
    async fn apply_auth_header(
        &self,
        builder: reqwest::RequestBuilder,
    ) -> bindle::client::Result<reqwest::RequestBuilder> {
        self.token_manager.apply_auth_header(builder).await
    }
}
