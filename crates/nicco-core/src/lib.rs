//! Read-only observations. No adapter exposes network mutation operations.
#[cfg(target_os = "linux")]
mod linux;
mod model;

pub use model::{Address, BackendEvidence, Interface, Snapshot, Source};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("live status is supported only on Linux; use --demo for synthetic data")]
    UnsupportedPlatform,
    #[error("network observation failed: {0}")]
    Observation(String),
    #[error("network observation timed out after 5 seconds")]
    Timeout,
    #[error("invalid demo fixture: {0}")]
    Fixture(#[from] serde_json::Error),
}

/// Collection is bounded; demo never inspects the host.
pub async fn collect(demo: bool) -> Result<Snapshot, Error> {
    if demo {
        return Snapshot::demo();
    }
    #[cfg(target_os = "linux")]
    {
        tokio::time::timeout(std::time::Duration::from_secs(5), linux::collect())
            .await
            .map_err(|_| Error::Timeout)?
    }
    #[cfg(not(target_os = "linux"))]
    {
        Err(Error::UnsupportedPlatform)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn demo_is_explicit_and_versioned() {
        let snapshot = collect(true).await.unwrap();
        assert_eq!(snapshot.schema_version, 1);
        assert_eq!(snapshot.source, Source::Demo);
        assert!(snapshot.interfaces.iter().all(|i| i.owner.is_none()));
        let value = serde_json::to_value(&snapshot).unwrap();
        assert_eq!(value["source"], "demo");
        assert_eq!(value["interfaces"][0]["addresses"][0]["prefix_len"], 24);
        let decoded: Snapshot = serde_json::from_value(value).unwrap();
        assert_eq!(decoded, snapshot);
    }

    #[cfg(not(target_os = "linux"))]
    #[tokio::test]
    async fn live_on_other_platforms_is_an_error() {
        assert!(matches!(
            collect(false).await,
            Err(Error::UnsupportedPlatform)
        ));
    }
}
