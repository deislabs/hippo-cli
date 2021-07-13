use itertools::Itertools;

pub trait ParcelHelpers {
    fn requires(&self) -> Vec<String>;
    fn is_member_of(&self, group: &str) -> bool;
}

pub trait InvoiceHelpers {
    fn parcels_in(&self, group: &str) -> Vec<bindle::Parcel>;
    fn parcels_required_by(&self, parcel: &bindle::Parcel) -> Vec<bindle::Parcel>;
}

impl ParcelHelpers for bindle::Parcel {
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