use std::{collections::HashMap, pin::Pin};

use bollard::Docker;
use color_eyre::eyre::Error;
use futures::{future::ready, Future, FutureExt};

use crate::{
    container, Container, ContainerBuilder, ContainerNetwork, ContainerNetworkBuilder, Image,
};

pub struct ContainerFut<'a, O = ()> {
    fut: Pin<Box<dyn Future<Output = O>>>,
    container: ContainerBuilder<'a>,
}

impl<'a> ContainerBuilder<'a> {
    pub fn perform(self) -> ContainerFut<'a> {
        ContainerFut::new(self)
    }
}

impl<'a> ContainerFut<'a> {
    pub fn new(container_builder: ContainerBuilder<'a>) -> ContainerFut<'a> {
        ContainerFut {
            fut: ready(()).boxed_local(),
            container: container_builder,
        }
    }
}

impl<'a, O> ContainerFut<'a, O> {
    pub fn map<F, A>(self, f: F) -> ContainerFut<'a, A>
    where
        O: 'static,
        A: 'static,
        F: FnOnce(O) -> A + 'static,
    {
        let new_fut = self.fut.map(f);
        ContainerFut {
            fut: new_fut.boxed_local(),
            container: self.container,
        }
    }

    pub fn then<F, Fut>(self, f: F) -> ContainerFut<'a, Fut::Output>
    where
        O: 'static,
        F: FnOnce(O) -> Fut + 'static,
        Fut: Future + 'static,
    {
        let new_fut = self.fut.then(f);
        ContainerFut {
            fut: new_fut.boxed_local(),
            container: self.container,
        }
    }
}

#[must_use = "Bootstrapper doesn't run if not consumed"]
pub struct BootstrapDockerNet<'a> {
    name: &'a str,
    container_futs: HashMap<usize, ContainerFut<'a>>,
}

impl<'a> BootstrapDockerNet<'a> {
    pub fn new(name: &'a str, containers: impl IntoIterator<Item = ContainerFut<'a>>) -> Self {
        Self {
            name,
            container_futs: containers.into_iter().enumerate().collect(),
        }
    }

    pub async fn run<E>(mut self, docker: impl FnOnce() -> Result<Docker, E>) -> Result<(), Error>
    where
        E: std::error::Error + Send + Sync + 'static,
    {
        match Runner::from_env() {
            Runner::ContainerId(id) => self.container_futs.remove(&id).unwrap().fut.await,
            Runner::Master => self.master_run(&docker()?).await?,
        };
        Ok(())
    }

    async fn master_run(self, docker: &Docker) -> Result<(), Error> {
        let net_builder = ContainerNetworkBuilder::new(self.name);
        let containers = self.container_futs.into_iter().map(|(id, c)| {
            c.container
                .with_env(RUNNER_ENV_VAR, id.to_string())
                .with_bootstrap()
        });
        net_builder
            .with_containers(containers)
            .build(docker)
            .await?
            .run(docker)
            .await
    }
}

enum Runner {
    ContainerId(usize),
    Master,
}

const RUNNER_ENV_VAR: &str = "__RUNNER_ENV";
impl Runner {
    fn from_env() -> Runner {
        std::env::var(RUNNER_ENV_VAR)
            .map(|x| Runner::ContainerId(x.parse().unwrap()))
            .unwrap_or(Runner::Master)
    }
}
