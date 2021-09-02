use itertools::Itertools;

use bindle::client::{Client, ClientBuilder};

pub struct BindleConnectionInfo {
    base_url: String,
    allow_insecure: bool,
    username: Option<String>,
    password: Option<String>,
}

impl BindleConnectionInfo {
    pub fn new<I: Into<String>>(base_url: I, allow_insecure: bool) -> Self {
        Self {
            base_url: base_url.into(),
            allow_insecure,
            username: None,
            password: None,
        }
    }

    pub fn set_username_password<I: Into<String>>(&mut self, username: I, password: I) -> Self {
        BindleConnectionInfo {
            base_url: self.base_url.clone(),
            username: Some(username.into()),
            password: Some(password.into()),
            allow_insecure: self.allow_insecure,
        }
    }

    pub fn client(&self) -> bindle::client::Result<Client> {
        let builder = bindle::client::ClientBuilder::default()
            .http2_prior_knowledge(false)
            .danger_accept_invalid_certs(self.allow_insecure);
        let auth = if let Some(username) = self.username {
            //panic!("Once the PR is merged, we can set the HTTP Basic auth");
            builder = builder.user_password(username, self.password.unwrap_or_default())
        };

        Ok(builder.build(&self.base_url)?)
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
