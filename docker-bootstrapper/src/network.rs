use bollard::{container::LogOutput, network::CreateNetworkOptions, Docker};
use color_eyre::{eyre::Error, owo_colors::OwoColorize};
use futures::{
    future::{select, try_join, try_select},
    pin_mut,
    stream::{self, FuturesUnordered},
    Future, FutureExt, SinkExt, Stream, StreamExt, TryFutureExt, TryStreamExt,
};

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
    pub async fn run(&self, docker: &Docker) -> Result<(), Error> {
        let start = self.start(docker);
        let log = self.log(docker, true);
        let wait = self.wait(docker);
        let flash = self.log(docker, false);

        pin_mut!(log);
        pin_mut!(wait);

        use futures::future::Either::*;
        let log_or_wait = select(log, wait).map(|either| match either {
            Left((x, _)) => {
                println!("log ends");
                x
            }
            Right((x, _)) => {
                println!("waiting ends");
                x
            }
        });

        // always run remove
        let task: Result<_, Error> = try {
            start.await?;
            log_or_wait.await?;
            flash.await?;
        };
        task.and(self.rm(docker).await)
    }

    pub async fn start(&self, docker: &Docker) -> Result<(), Error> {
        self.containers
            .iter()
            .map(|c| c.start(docker))
            .collect::<FuturesUnordered<_>>()
            .try_collect::<()>()
            .await
    }

    pub async fn log(&self, docker: &Docker, follow: bool) -> Result<(), Error> {
        stream::iter(
            self.containers
                .iter()
                .map(|c| c.log(docker, follow).map_ok(move |x| (c, x))),
        )
        .flatten()
        .try_for_each_concurrent(None, |(c, l)| async move {
            println!("{}: {}", c.name(), l.to_string().trim());
            Ok(())
        })
        .await
    }

    pub async fn wait(&self, docker: &Docker) -> Result<(), Error> {
        self.containers
            .iter()
            .filter(|c| c.is_waited)
            .map(|c| c.wait(docker))
            .collect::<FuturesUnordered<_>>()
            .try_collect()
            .await
    }

    pub async fn rm(&self, docker: &Docker) -> Result<(), Error> {
        self.containers
            .iter()
            .map(|c| c.rm(docker))
            .collect::<FuturesUnordered<_>>()
            .try_collect::<()>()
            .await?;

        docker.remove_network(&self.id).await?;
        Ok(())
    }
}
