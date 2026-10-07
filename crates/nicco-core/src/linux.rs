use crate::{Address, BackendEvidence, Error, Interface, Snapshot, Source};
use futures_util::TryStreamExt;
use rtnetlink::packet_route::{address::AddressAttribute, link::LinkAttribute};

// Aborting on drop prevents a background netlink connection from outliving a
// failed or timed-out collection. Every request below is a GET dump.
struct Connection(tokio::task::JoinHandle<()>);
impl Drop for Connection {
    fn drop(&mut self) {
        self.0.abort();
    }
}

pub async fn collect() -> Result<Snapshot, Error> {
    let (connection, handle, _) =
        rtnetlink::new_connection().map_err(|e| Error::Observation(e.to_string()))?;
    let _connection = Connection(tokio::spawn(connection));
    let mut interfaces = Vec::new();
    let mut links = handle.link().get().execute();
    while let Some(link) = links.try_next().await.map_err(observation)? {
        let mut name = None;
        let mut oper_state = "unknown".to_owned();
        for attr in link.attributes {
            match attr {
                LinkAttribute::IfName(value) => name = Some(value),
                LinkAttribute::OperState(value) => oper_state = format!("{value:?}").to_lowercase(),
                _ => {}
            }
        }
        interfaces.push(Interface {
            index: link.header.index,
            name: name.unwrap_or_else(|| format!("ifindex:{}", link.header.index)),
            oper_state,
            addresses: Vec::new(),
            owner: None,
        });
    }
    let mut addresses = handle.address().get().execute();
    while let Some(message) = addresses.try_next().await.map_err(observation)? {
        // For point-to-point IPv4, Local is our address and Address may be the peer.
        let local = message.attributes.iter().find_map(|a| match a {
            AddressAttribute::Local(ip) => Some(*ip),
            _ => None,
        });
        let ip = local.or_else(|| {
            message.attributes.iter().find_map(|a| match a {
                AddressAttribute::Address(ip) => Some(*ip),
                _ => None,
            })
        });
        if let (Some(ip), Some(interface)) = (
            ip,
            interfaces
                .iter_mut()
                .find(|i| i.index == message.header.index),
        ) {
            interface.addresses.push(Address {
                address: ip,
                prefix_len: message.header.prefix_len,
            });
        }
    }
    let mut snapshot = Snapshot {
        schema_version: 1,
        source: Source::Live,
        interfaces,
        backends: [ ("NetworkManager", "/run/NetworkManager"), ("systemd-networkd", "/run/systemd/netif") ]
            .into_iter().map(|(backend, path)| BackendEvidence {
                backend: backend.into(),
                runtime_directory_present: std::path::Path::new(path).is_dir(),
                limitation: "Directory presence does not prove availability, service health, capabilities or interface ownership".into(),
            }).collect(),
    };
    snapshot.normalize();
    Ok(snapshot)
}

fn observation(error: rtnetlink::Error) -> Error {
    Error::Observation(error.to_string())
}
