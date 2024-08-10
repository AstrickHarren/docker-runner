use bollard::{network::CreateNetworkOptions, Docker};
use color_eyre::{
    eyre::Error,
    owo_colors::{OwoColorize, Style},
};
use futures::{
    future::select_all,
    stream::{self, FuturesUnordered},
    FutureExt, StreamExt, TryFutureExt, TryStreamExt,
};

use crate::{utils::ctrl_c, Container, ContainerBuilder};

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
        let cancel = ctrl_c()
            .map_err(|e| e.into())
            .inspect_ok(|_| Self::print_cancel_msg());

        let log_wait_ctrlc =
            select_all([log.boxed_local(), wait.boxed_local(), cancel.boxed_local()].into_iter())
                .map(|(x, _, _)| x);

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
        // NOTE: should I use try_for_each_concurrent instead?
        .try_fold(Dialogger::default(), |mut logger, (c, l)| async move {
            match l.to_string().trim() {
                "" => Ok(logger),
                l => {
                    logger.log(c, l.to_string().trim());
                    Ok(logger.with_id(c))
                }
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

#[derive(Default)]
struct Dialogger<'a> {
    dia_len: usize,
    current_id: Option<&'a Container>,
}

impl<'a> Dialogger<'a> {
    fn print_start(id: &Container, msg: &str) {
        println!("{:<20}{}", id.name(), msg)
    }

    fn print_mid(msg: &str) {
        println!("{:<20}{}", " ┃", msg)
    }

    fn print_end(&self) {
        if self.dia_len > 0 {
            println!(" ┗━━");
        }
    }

    fn with_id(self, id: &'a Container) -> Self {
        let dia_len = if self.current_id == Some(id) {
            self.dia_len + 1
        } else {
            0
        };

        Self {
            dia_len,
            current_id: id.into(),
        }
    }

    fn log(&mut self, id: &Container, msg: &str) {
        match self.current_id {
            Some(x) if &x == &id => Self::print_mid(msg),
            Some(_) => {
                self.print_end();
                Self::print_start(id, msg)
            }
            None => Self::print_start(id, msg),
        };
    }
}
