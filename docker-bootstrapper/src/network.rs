use bollard::{network::CreateNetworkOptions, Docker};
use color_eyre::eyre::Error;
use futures::{stream::FuturesUnordered, FutureExt, StreamExt, TryStreamExt};

use crate::{Container, ContainerBuilder};

pub struct ContainerNetworkBuilder<'a> {
    opts: CreateNetworkOptions<&'a str>,
    containers: Vec<ContainerBuilder<'a>>,
}

impl<'a> ContainerNetworkBuilder<'a> {
    pub fn new(name: &'a str) -> Self {
        Self {
            opts: CreateNetworkOptions {
                name,
                check_duplicate: true,
                driver: "bridge",
                internal: true,
                // enable_ipv6: true,
                ..Default::default()
            },
            containers: Default::default(),
        }
    }

    pub fn add_container(&mut self, container: ContainerBuilder<'a>) {
        self.containers.push(container);
    }

    pub fn with_containers(
        mut self,
        containers: impl IntoIterator<Item = ContainerBuilder<'a>>,
    ) -> Self {
        self.containers.extend(containers);
        self
    }

    pub async fn build(self, docker: &Docker) -> Result<ContainerNetwork, Error> {
        // 1. create network
        let network = docker.create_network(self.opts).await?;
        network.warning.inspect(|x| eprintln!("{}", x));
        let network_id = network.id.unwrap();

        // 2. create containers
        let containers: Vec<_> = self
            .containers
            .into_iter()
            .map(|container| container.with_net(&network_id).build(docker))
            .collect::<FuturesUnordered<_>>()
            .try_collect()
            .await?;

        Ok(ContainerNetwork {
            id: network_id,
            containers,
        })
    }
}

pub struct ContainerNetwork {
    id: String,
    containers: Vec<Container>,
}

impl ContainerNetwork {
    pub async fn run(&self, docker: &Docker) -> Result<(), Error> {
        self.containers
            .iter()
            .map(|c| c.run(docker))
            .collect::<FuturesUnordered<_>>()
            .try_collect::<()>()
            .await?;
        self.rm(docker).await?;
        Ok(())
    }

    pub async fn rm(&self, docker: &Docker) -> Result<(), Error> {
        docker.remove_network(&self.id).await?;
        Ok(())
    }
}
