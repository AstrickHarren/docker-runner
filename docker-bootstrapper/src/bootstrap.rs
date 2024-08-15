use std::{borrow::Cow, collections::HashMap, pin::Pin};

use bollard::Docker;
use color_eyre::eyre::Error;
use futures::{future::ready, Future, FutureExt};

use crate::{ContainerBuilder, ContainerNetworkBuilder};

pub struct ContainerFut<'a, T, O = ()> {
    fut: Pin<Box<dyn Future<Output = O>>>,
    container: ContainerBuilder<'a, T>,
}

impl<'a, T> ContainerBuilder<'a, T> {
    pub fn start(self) -> ContainerFut<'a, T> {
        ContainerFut::new(self)
    }

    pub fn start_with<O>(self, code: impl Future<Output = O> + 'static) -> ContainerFut<'a, T, O> {
        ContainerFut {
            fut: code.boxed_local(),
            container: self,
        }
    }
}

impl<'a, T> ContainerFut<'a, T> {
    pub fn new(container_builder: ContainerBuilder<'a, T>) -> ContainerFut<'a, T> {
        ContainerFut {
            fut: ready(()).boxed_local(),
            container: container_builder,
        }
    }
}

impl<'a, T, O> ContainerFut<'a, T, O> {
    pub fn map<F, A>(self, f: F) -> ContainerFut<'a, T, A>
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

    pub fn then<F, Fut>(self, f: F) -> ContainerFut<'a, T, Fut::Output>
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
pub struct BootstrapDockerNet<'a, T> {
    name: &'a str,
    container_futs: HashMap<usize, ContainerFut<'a, T>>,
}

impl<'a, T> BootstrapDockerNet<'a, T> {
    pub fn new(name: &'a str, containers: impl IntoIterator<Item = ContainerFut<'a, T>>) -> Self {
        Self {
            name,
            container_futs: containers.into_iter().enumerate().collect(),
        }
    }

    pub async fn run<'b, E>(
        mut self,
        docker: impl FnOnce() -> Result<Docker, E>,
    ) -> Result<(), Error>
    where
        E: std::error::Error + Send + Sync + 'static,
        T: Into<Cow<'b, str>>,
    {
        match Runner::from_env() {
            Runner::ContainerId(id) => self.container_futs.remove(&id).unwrap().fut.await,
            Runner::Master => self.master_run(&docker()?).await?,
        };
        Ok(())
    }

    async fn master_run<'b>(self, docker: &Docker) -> Result<(), Error>
    where
        T: Into<Cow<'b, str>>,
    {
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
