pub trait ParcelHelpers {
    fn requires(&self) -> Vec<String>;
    fn is_member_of(&self, group: &str) -> bool;
}

pub trait InvoiceHelpers {
    fn parcels_in(&self, group: &str) -> Vec<bindle::Parcel>;
}

impl ParcelHelpers for bindle::Parcel {
    fn requires(&self) -> Vec<String> {
        match self.conditions.as_ref() {
            None => vec![],
            Some(conditions) => match conditions.requires.as_ref() {
                None => vec![],
                Some(groups) => groups.clone(),
            }
        }
    }

    fn is_member_of(&self, group: &str) -> bool {
        match self.conditions.as_ref() {
            None => false,
            Some(conditions) => match conditions.member_of.as_ref() {
                None => false,
                Some(groups) => groups.contains(&group.to_owned()),
            }
        }
    }
}

impl InvoiceHelpers for bindle::Invoice {
    fn parcels_in(&self, group: &str) -> Vec<bindle::Parcel> {
        match self.parcel.as_ref() {
            None => vec![],
            Some(parcels) => parcels.iter().filter(|p| p.is_member_of(group)).cloned().collect(),
        }
    }
}
