/// Docker client wrapper for reach sandbox management.
///
/// Uses bollard to communicate with the Docker Engine API.
/// Handles container CRUD, health polling, and label-based discovery.

pub struct DockerClient {
    client: bollard::Docker,
}

impl DockerClient {
    pub fn new() -> anyhow::Result<Self> {
        let client = bollard::Docker::connect_with_local_defaults()?;
        Ok(Self { client })
    }

    pub fn inner(&self) -> &bollard::Docker {
        &self.client
    }
}
