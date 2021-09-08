use itertools::Itertools;
use std::collections::HashMap;
use std::path::Path;

use bindle::client::{
    tokens::{HttpBasic, NoToken, TokenManager},
    Client, ClientBuilder,
};

use crate::hippofacts::{HippoFacts, HippoFactsEntry};

enum AuthMethod {
    HttpBasic(HttpBasic),
    None(NoToken),
}

pub struct BindleConnectionInfo {
    base_url: String,
    allow_insecure: bool,
    auth_method: AuthMethod,
}

impl BindleConnectionInfo {
    pub fn new<I: Into<String>>(
        base_url: I,
        allow_insecure: bool,
        username: Option<String>,
        password: Option<String>,
    ) -> Self {
        let auth_method = match (username, password) {
            (Some(u), Some(p)) => AuthMethod::HttpBasic(HttpBasic::new(&u, &p)),
            _ => AuthMethod::None(NoToken::default()),
        };

        Self {
            base_url: base_url.into(),
            allow_insecure,
            auth_method,
        }
    }

    pub fn client<T: TokenManager>(&self, token_manager: T) -> bindle::client::Result<Client<T>> {
        let builder = ClientBuilder::default()
            .http2_prior_knowledge(false)
            .danger_accept_invalid_certs(self.allow_insecure);
        builder.build(&self.base_url, token_manager)
    }

    pub async fn push_all(
        &self,
        path: impl AsRef<Path>,
        bindle_id: &bindle::Id,
    ) -> anyhow::Result<()> {
        let reader = bindle::standalone::StandaloneRead::new(&path, bindle_id).await?;

        match &self.auth_method {
            AuthMethod::HttpBasic(token_manager) => {
                let builder = ClientBuilder::default()
                    .http2_prior_knowledge(false)
                    .danger_accept_invalid_certs(self.allow_insecure);
                let client = builder.build(&self.base_url, token_manager.clone())?;
                reader
                    .push(&client)
                    .await
                    .map_err(|e| anyhow::anyhow!("Error pushing bindle to server: {}", e))?;
                Ok(())
            }
            AuthMethod::None(token_manager) => {
                let builder = ClientBuilder::default()
                    .http2_prior_knowledge(false)
                    .danger_accept_invalid_certs(self.allow_insecure);
                let client = builder.build(&self.base_url, token_manager.clone())?;
                reader
                    .push(&client)
                    .await
                    .map_err(|e| anyhow::anyhow!("Error pushing bindle to server: {}", e))?;
                Ok(())
            }
        }
    }

    pub async fn prefetch_required_invoices(
        &self,
        hippofacts: &HippoFacts,
    ) -> anyhow::Result<HashMap<bindle::Id, bindle::Invoice>> {
        let mut map = HashMap::new();
        let external_refs: Vec<bindle::Id> = hippofacts
            .entries
            .iter()
            .flat_map(external_bindle_id)
            .collect();
        if external_refs.is_empty() {
            return Ok(map);
        }

        match &self.auth_method {
            AuthMethod::HttpBasic(token_manager) => {
                let builder = ClientBuilder::default()
                    .http2_prior_knowledge(false)
                    .danger_accept_invalid_certs(self.allow_insecure);
                let client = builder.build(&self.base_url, token_manager.clone())?;
                for external_ref in external_refs {
                    let invoice = client.get_yanked_invoice(&external_ref).await?;
                    map.insert(external_ref, invoice);
                }

                Ok(map)
            }
            AuthMethod::None(token_manager) => {
                let builder = ClientBuilder::default()
                    .http2_prior_knowledge(false)
                    .danger_accept_invalid_certs(self.allow_insecure);
                let client = builder.build(&self.base_url, token_manager.clone())?;
                for external_ref in external_refs {
                    let invoice = client.get_yanked_invoice(&external_ref).await?;
                    map.insert(external_ref, invoice);
                }

                Ok(map)
            }
        }
    }
}

pub trait ParcelHelpers {
    fn has_annotation(&self, key: &str) -> bool;
    fn requires(&self) -> Vec<String>;
    fn is_member_of(&self, group: &str) -> bool;
}

pub trait InvoiceHelpers {
    fn parcels_in(&self, group: &str) -> Vec<bindle::Parcel>;
    fn parcels_required_by(&self, parcel: &bindle::Parcel) -> Vec<bindle::Parcel>;
}

impl ParcelHelpers for bindle::Parcel {
    fn has_annotation(&self, key: &str) -> bool {
        match self.label.annotations.as_ref() {
            None => false,
            Some(map) => map.contains_key(key),
        }
    }

    fn requires(&self) -> Vec<String> {
        match self.conditions.as_ref() {
            None => vec![],
            Some(conditions) => match conditions.requires.as_ref() {
                None => vec![],
                Some(groups) => groups.clone(),
            },
        }
    }

    fn is_member_of(&self, group: &str) -> bool {
        match self.conditions.as_ref() {
            None => false,
            Some(conditions) => match conditions.member_of.as_ref() {
                None => false,
                Some(groups) => groups.contains(&group.to_owned()),
            },
        }
    }
}

impl InvoiceHelpers for bindle::Invoice {
    fn parcels_in(&self, group: &str) -> Vec<bindle::Parcel> {
        match self.parcel.as_ref() {
            None => vec![],
            Some(parcels) => parcels
                .iter()
                .filter(|p| p.is_member_of(group))
                .cloned()
                .collect(),
        }
    }

    fn parcels_required_by(&self, parcel: &bindle::Parcel) -> Vec<bindle::Parcel> {
        parcels_required_by_acc(self, parcel.requires(), vec![])
            .into_iter()
            .unique_by(|p| p.label.sha256.clone())
            .collect_vec()
    }
}

fn parcels_required_by_acc(
    invoice: &bindle::Invoice,
    mut groups: Vec<String>,
    mut acc: Vec<bindle::Parcel>,
) -> Vec<bindle::Parcel> {
    match groups.pop() {
        None => acc,
        Some(group) => {
            let mut members = invoice.parcels_in(&group);
            let mut required_groups: Vec<_> =
                members.iter().flat_map(|p| p.requires()).unique().collect();
            acc.append(&mut members);
            groups.append(&mut required_groups);
            let new_groups = groups.into_iter().unique().collect();
            parcels_required_by_acc(invoice, new_groups, acc)
        }
    }
}

fn external_bindle_id(entry: &HippoFactsEntry) -> Option<bindle::Id> {
    entry.external_ref().map(|ext| ext.bindle_id)
}
