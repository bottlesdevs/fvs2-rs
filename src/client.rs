use std::{
    path::Path,
    process::{Child, Command, Stdio},
    time::Duration,
};

use futures_timer::Delay;
use tonic::{
    Status,
    transport::{Channel, Endpoint},
};
use tonic_health::pb::{
    HealthCheckRequest, health_check_response::ServingStatus, health_client::HealthClient,
};

use crate::{
    error::{Error, Result},
    proto::{
        self, Commit, CommitRequest, CreateMountRequest, GetMountRequest, InitRepositoryRequest,
        Layer, ListCommitsRequest, Mount, MountSpec, Repository, RestoreRequest, RestoreResponse,
        ShutdownRequest, UnmountMode, UnmountRequest, fvs2d_client::Fvs2dClient as GrpcClient,
    },
};

const HEALTH_POLL_INTERVAL: Duration = Duration::from_millis(100);

/// A reconnectable client for an `fvs2d` daemon.
pub struct Fvs2dClient {
    client: GrpcClient<Channel>,
}

impl Fvs2dClient {
    /// Connect to a daemon already serving on `socket`.
    pub async fn connect(socket: impl AsRef<Path>) -> Result<Self> {
        let endpoint = endpoint(socket.as_ref())?;
        let channel = endpoint.connect().await?;
        check_health(&channel).await?;
        Ok(Self {
            client: GrpcClient::new(channel),
        })
    }

    /// Connect to the stable `socket`, spawning the daemon when necessary.
    ///
    /// The spawned process is deliberately detached from this client so it can
    /// serve later clients after this one is dropped.
    pub async fn connect_or_spawn(
        executable: impl AsRef<Path>,
        socket: impl AsRef<Path>,
    ) -> Result<Self> {
        let executable = executable.as_ref();
        let socket = socket.as_ref();

        if let Ok(client) = Self::connect(socket).await {
            return Ok(client);
        }

        if !executable.is_file() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("{} does not exist", executable.display()),
            )
            .into());
        }

        if let Some(parent) = socket.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let control = format!("unix:{}", socket.display());

        let mut child = Command::new(executable)
            .arg("-control")
            .arg(&control)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()?;

        let endpoint = Endpoint::from_shared(control)?;
        let channel = connect_ready(&endpoint, &mut child).await?;

        Ok(Self {
            client: GrpcClient::new(channel),
        })
    }

    /// Initialize an FVS repository at `path` with the given `block_size`.
    ///
    /// `path` must already exist and be a directory.
    pub async fn new_repository(
        &self,
        path: impl AsRef<Path>,
        block_size: u32,
    ) -> Result<Repository> {
        let mut client = self.client.clone();
        let path = path.as_ref();

        if !path.exists() {
            return Err(Status::not_found("Path does not exist").into());
        }

        if !path.is_dir() {
            return Err(Status::internal("Path must be a directory").into());
        }

        let resp = client
            .init_repository(InitRepositoryRequest {
                repository_path: path.display().to_string(),
                block_size,
            })
            .await?;

        Ok(resp.into_inner())
    }

    /// Commit the current working tree of `repository` with `message`.
    pub async fn commit(&self, repository: &Repository, message: String) -> Result<Commit> {
        let mut client = self.client.clone();

        let resp = client
            .commit(CommitRequest {
                message,
                allow_empty: false,
                repository_path: repository.repository_path.clone(),
            })
            .await?;

        Ok(resp.into_inner())
    }

    /// List commits in `repository`, newest-first as returned by the daemon.
    pub async fn list_commits(&self, repository: &Repository) -> Result<Vec<Commit>> {
        let mut client = self.client.clone();

        let resp = client
            .list_commits(ListCommitsRequest {
                repository_path: repository.repository_path.clone(),
            })
            .await?;

        Ok(resp.into_inner().commits)
    }

    /// Restore `commit` into `repository`.
    ///
    /// - `destination`: override restore target (default: repo root)
    /// - `clean`: remove files in the destination that are not in the commit
    /// - `reset`: move HEAD to the restored commit
    pub async fn restore(
        &self,
        repository: &Repository,
        commit: &Commit,
        destination: Option<impl AsRef<Path>>,
        clean: bool,
        reset: bool,
    ) -> Result<RestoreResponse> {
        let mut client = self.client.clone();

        let resp = client
            .restore(RestoreRequest {
                repository_path: repository.repository_path.to_string(),
                state_id_or_prefix: commit.state_id.to_string(),
                destination_path: destination.map(|dest| dest.as_ref().display().to_string()),
                clean,
                reset,
            })
            .await?;

        Ok(resp.into_inner())
    }

    /// Mount `layers` at `mount_point`, optionally with a writable `upper` dir.
    pub async fn mount(
        &self,
        mount_point: impl AsRef<Path>,
        layers: Vec<Layer>,
        upper: Option<impl AsRef<Path>>,
    ) -> Result<Mount> {
        let mut client = self.client.clone();

        let resp = client
            .create_mount(CreateMountRequest {
                spec: Some(MountSpec {
                    mount_point: mount_point.as_ref().display().to_string(),
                    layers,
                    upper_path: upper.map(|path| path.as_ref().display().to_string()),
                    debug: false,
                }),
            })
            .await?;

        Ok(resp.into_inner())
    }

    /// Unmount a previously created [`Mount`] using `mode`.
    pub async fn unmount(&self, mount: &Mount, mode: UnmountMode) -> Result<()> {
        let mut client = self.client.clone();

        client
            .unmount(UnmountRequest {
                mode: mode as i32,
                mount_id: mount.id.clone(),
            })
            .await?;

        Ok(())
    }

    /// Fetch a mount by id.
    pub async fn get_mount(&self, id: String) -> Result<Mount> {
        let mut client = self.client.clone();

        let resp = client.get_mount(GetMountRequest { mount_id: id }).await?;

        Ok(resp.into_inner())
    }

    /// List mounts currently held by this daemon.
    pub async fn list_mounts(&self) -> Result<Vec<Mount>> {
        let mut client = self.client.clone();

        let resp = client.list_mounts(()).await?;

        Ok(resp.into_inner().mounts)
    }

    /// Ask the daemon to shut down, unmounting with `mode`, then drop `self`.
    pub async fn shutdown(mut self, mode: UnmountMode) -> Result<()> {
        self.client
            .shutdown(ShutdownRequest { mode: mode as i32 })
            .await?;

        Ok(())
    }
}

fn endpoint(socket: &Path) -> Result<Endpoint> {
    Ok(Endpoint::from_shared(format!("unix:{}", socket.display()))?)
}

async fn check_health(channel: &Channel) -> Result<()> {
    let request = HealthCheckRequest {
        service: proto::fvs2d_server::SERVICE_NAME.to_string(),
    };
    let response = HealthClient::new(channel.clone()).check(request).await?;
    if response.get_ref().status() != ServingStatus::Serving {
        return Err(Status::unavailable("fvs2d is not serving").into());
    }
    Ok(())
}

/// Poll until the child is serving or has exited.
async fn connect_ready(endpoint: &Endpoint, child: &mut Child) -> Result<Channel> {
    loop {
        if let Some(status) = child.try_wait()? {
            return Err(Error::ProcessExit(status));
        }

        let channel = match endpoint.connect().await {
            Ok(channel) => channel,
            Err(_) => {
                Delay::new(HEALTH_POLL_INTERVAL).await;
                continue;
            }
        };

        match check_health(&channel).await {
            Ok(()) => return Ok(channel),
            Err(_) => Delay::new(HEALTH_POLL_INTERVAL).await,
        }
    }
}
