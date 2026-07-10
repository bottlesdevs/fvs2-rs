use thiserror::Error;

pub(crate) type Result<T> = std::result::Result<T, Error>;

#[derive(Error, Debug)]
pub enum Error {
    #[error("I/O: {0}")]
    Io(#[from] std::io::Error),
    #[error("Tonic Transport: {0}")]
    Transport(#[from] tonic::transport::Error),
    #[error("Tonic Status: {0}")]
    Status(#[from] tonic::Status),
    #[error("fvs2d exited unexpectedly with status {0}")]
    ProcessExit(std::process::ExitStatus),
}
