pub mod client;
pub mod error;

mod proto {
    tonic::include_proto!("fvs2d.v1");

    impl Layer {
        pub fn new(repository: &Repository, commit: Option<&Commit>) -> Self {
            Layer {
                repository_path: repository.repository_path.clone(),
                revision: commit.map(|commit| CommitSelector {
                    selector: Some(commit_selector::Selector::StateIdOrPrefix(
                        commit.state_id.clone(),
                    )),
                }),
            }
        }
    }
}

pub use client::Fvs2dClient;
pub use prost_types::Timestamp;
pub use proto::{Commit, Layer, Mount, Repository, RestoreResponse, UnmountMode};

#[cfg(test)]
mod tests {}
