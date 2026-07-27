pub mod client;
pub mod error;

mod proto {
    tonic::include_proto!("fvs2d.v1");
}

pub use client::Fvs2dClient;
pub use prost_types::Timestamp;
pub use proto::{
    ChangeKind, Commit, CommitSummary, FileChange, Layer, Mount, Progress, Repository,
    RestoreResponse, UnmountMode,
};

impl Layer {
    pub fn new(repository: &Repository, commit: Option<&Commit>) -> Self {
        Self::from_state_id(repository, commit.map(|commit| commit.state_id.as_str()))
    }

    pub fn from_summary(repository: &Repository, commit: Option<&CommitSummary>) -> Self {
        Self::from_state_id(repository, commit.map(|commit| commit.state_id.as_str()))
    }

    fn from_state_id(repository: &Repository, state_id: Option<&str>) -> Self {
        Layer {
            repository_path: repository.repository_path.clone(),
            revision: state_id.map(|state_id| crate::proto::CommitSelector {
                selector: Some(crate::proto::commit_selector::Selector::StateIdOrPrefix(
                    state_id.to_owned(),
                )),
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layer_uses_listed_commit_summary() {
        let repository = Repository {
            repository_path: "/repo".into(),
            block_size: 4096,
        };
        let commit = CommitSummary {
            state_id: "abc123".into(),
            ..Default::default()
        };

        assert_eq!(
            Layer::from_summary(&repository, Some(&commit))
                .revision
                .unwrap()
                .selector,
            Some(crate::proto::commit_selector::Selector::StateIdOrPrefix(
                "abc123".into()
            ))
        );
    }
}
