use bollard::{container::LogOutput, network::CreateNetworkOptions, Docker};
use color_eyre::eyre::Error;
use futures::{stream::FuturesUnordered, FutureExt, Stream, StreamExt, TryStreamExt};

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

#[must_use = "network build is not removed if not used"]
pub struct ContainerNetwork {
    id: String,
    containers: Vec<Container>,
}

impl ContainerNetwork {
    pub async fn run(self, docker: &Docker) -> Result<(), Error> {
        let result = self
            .start_and_attach(docker)
            .map_ok(|(c, l)| print!("{:<20} {}", c.name(), l))
            .try_collect::<()>()
            .await;
        self.rm(docker).await?;
        result
    }

    pub async fn rm(self, docker: &Docker) -> Result<(), Error> {
        self.containers
            .iter()
            .map(|container| async {
                if container.is_waited {
                    container.wait(docker).await?;
                }
                container.rm(docker).await?;
                Ok::<_, Error>(())
            })
            .collect::<FuturesUnordered<_>>()
            .try_for_each_concurrent(None, |_| async { Ok::<_, Error>(()) })
            .await?;

        docker.remove_network(&self.id).await?;
        Ok(())
    }

    fn start_and_attach<'a>(
        &'a self,
        docker: &'a Docker,
    ) -> impl Stream<Item = Result<(&Container, LogOutput), Error>> + 'a {
        let streams = self
            .containers
            .iter()
            .map(|c| {
                c.start(docker)
                    .map(|_| c.log(docker))
                    .map(move |log| log.map_ok(move |x| (c, x)))
            })
            .collect::<FuturesUnordered<_>>()
            .flatten();

        streams
    }
}
