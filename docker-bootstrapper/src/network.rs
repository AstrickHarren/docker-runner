mod dialog;

use std::borrow::Cow;

use bollard::{network::CreateNetworkOptions, Docker};
use color_eyre::{
    eyre::Error,
    owo_colors::{OwoColorize, Style},
};
use dialog::Dialogger;
use futures::{
    future::select_all,
    stream::{self, FuturesUnordered},
    FutureExt, StreamExt, TryFutureExt, TryStreamExt,
};

use crate::{utils::ctrl_c, Container, ContainerBuilder};

pub struct ContainerNetworkBuilder<'a, T> {
    opts: CreateNetworkOptions<&'a str>,
    containers: Vec<ContainerBuilder<'a, T>>,
}

impl<'a, T> ContainerNetworkBuilder<'a, T> {
    pub fn new(name: &'a str) -> Self {
        Self {
            opts: CreateNetworkOptions {
                name,
                check_duplicate: true,
                driver: "bridge",
                // enable_ipv6: true,
                ..Default::default()
            },
            containers: Default::default(),
        }
    }

    pub fn add_container(&mut self, container: ContainerBuilder<'a, T>) {
        self.containers.push(container);
    }

    pub fn with_containers(
        mut self,
        containers: impl IntoIterator<Item = ContainerBuilder<'a, T>>,
    ) -> Self {
        self.containers.extend(containers);
        self
    }

    pub async fn build<'b>(self, docker: &Docker) -> Result<ContainerNetwork, Error>
    where
        T: Into<Cow<'b, str>>,
    {
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
        let log_again = self.log(docker, false);
        let cancel = ctrl_c()
            .map_err(|e| e.into())
            .inspect_ok(|_| Self::print_cancel_msg());

        let log_wait_ctrlc = select_all(
            [
                // Log and then wait to make sure if container exited normally
                log.and_then(|_| self.wait(docker)).boxed_local(),
                // At the same time, if container exited early, abort entire network
                self.wait(docker).boxed_local(),
                // At the same time, if user hit interrupt, abort network
                cancel.boxed_local(),
            ]
            .into_iter(),
        )
        .then(|(x, i, _)| async move {
            // if container exited early, print log again
            if i == 1 {
                println!(
                    "\n{}",
                    "=============== CONTAINER EXITED EARLY: LOGS ============= "
                        .red()
                        .bold()
                );
                log_again.await?;
            }
            x
        });

        let task: Result<_, Error> = try {
            start.await?;
            log_wait_ctrlc.await?;
        };

        // always run remove
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
        .flatten_unordered(None)
        .try_fold(Dialogger::default(), |logger, (c, l)| async move {
            match l.to_string().trim_matches('\n') {
                "" => Ok(logger),
                l => Ok(logger.log(c, l.to_string().trim_matches('\n'))),
            }
        })
        .await?
        .print_end();
        Ok(())
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

    fn print_cancel_msg() {
        let canceling = "Canceling".style(Style::new().red().bold());
        let interrupt = "interrupt".style(Style::new().red().bold());
        let docker = "docker".style(Style::new().blue().bold());
        let hint = "Hint".style(Style::new().yellow().bold());
        println!("\n\t{canceling} due to {interrupt}: cleaning lingering {docker} resources...");
        println!("\t{hint}: hit interrupt again to force quit");
    }
}
