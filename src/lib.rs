pub mod client;
pub mod error;

mod proto {
    tonic::include_proto!("fvs2d.v1");
}

pub use client::Fvs2dClient;
pub use prost_types::Timestamp;
pub use proto::{Commit, Layer, Mount, Repository, UnmountMode};

#[cfg(test)]
mod tests {}
