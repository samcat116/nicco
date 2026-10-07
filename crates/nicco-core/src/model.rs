use serde::{Deserialize, Serialize};
use std::net::IpAddr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Source {
    Live,
    Demo,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Snapshot {
    pub schema_version: u32,
    pub source: Source,
    pub interfaces: Vec<Interface>,
    pub backends: Vec<BackendEvidence>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Interface {
    pub index: u32,
    pub name: String,
    /// Kernel operational state, distinct from configuration/administrative state.
    pub oper_state: String,
    pub addresses: Vec<Address>,
    /// No ownership inference from daemon presence or directory existence.
    pub owner: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Address {
    pub address: IpAddr,
    pub prefix_len: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BackendEvidence {
    pub backend: String,
    pub runtime_directory_present: bool,
    pub limitation: String,
}

impl Snapshot {
    pub fn normalize(&mut self) {
        self.interfaces.sort_by_key(|i| i.index);
        for interface in &mut self.interfaces {
            interface.addresses.sort();
            interface.addresses.dedup();
        }
        self.backends.sort_by(|a, b| a.backend.cmp(&b.backend));
    }

    pub fn demo() -> Result<Self, crate::Error> {
        let mut snapshot: Self = serde_json::from_str(include_str!("../fixtures/demo.json"))?;
        snapshot.normalize();
        Ok(snapshot)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalization_orders_and_deduplicates() {
        let mut s = Snapshot::demo().unwrap();
        let address = s.interfaces[0].addresses[0].clone();
        s.interfaces[0].addresses.push(address);
        s.interfaces.reverse();
        s.normalize();
        assert_eq!(s.interfaces[0].index, 1);
        assert_eq!(s.interfaces[0].addresses.len(), 1);
    }

    #[test]
    fn json_contract_matches_fixture_exactly() {
        let snapshot = Snapshot::demo().unwrap();
        let expected: serde_json::Value =
            serde_json::from_str(include_str!("../fixtures/demo.json")).unwrap();
        assert_eq!(serde_json::to_value(snapshot).unwrap(), expected);
    }
}
