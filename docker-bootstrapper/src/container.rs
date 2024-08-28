use std::{
    borrow::Cow,
    env::{self},
    fmt::Display,
    path::Path,
};

use bollard::{
    container::{
        Config, CreateContainerOptions, LogOutput, LogsOptions, RemoveContainerOptions,
        WaitContainerOptions,
    },
    exec::{CreateExecOptions, StartExecResults},
    Docker,
};
use color_eyre::eyre::{eyre, Error};

use color_eyre::owo_colors::OwoColorize;

use futures::{Stream, TryStreamExt};

use crate::ImageBuilder;

impl<'a, T> ImageBuilder<T> {
    pub fn to_container(self, name: &'a str) -> ContainerBuilder<T> {
        ContainerBuilder::new(name, self)
    }
}

pub struct ContainerBuilder<'a, T> {
    image: ImageBuilder<T>,
    opts: CreateContainerOptions<&'a str>,
    config: Config<String>,
    /// If is waited for the docker network before it removes this container with it finishing its execution
    is_waited: bool,
}

impl<'a, T> ContainerBuilder<'a, T> {
    pub fn new(name: &'a str, image_builder: ImageBuilder<T>) -> Self {
        Self {
            opts: CreateContainerOptions {
                name,
                ..Default::default()
            },
            image: image_builder,
            config: Config {
                image: None,
                tty: Some(true),
                attach_stdout: Some(true),
                attach_stderr: Some(true),
                ..Default::default()
            },
            is_waited: false,
        }
    }

    pub fn with_cmd(mut self, cmd: impl IntoIterator<Item = impl ToString>) -> Self {
        self.config.cmd = Some(cmd.into_iter().map(|x| x.to_string()).collect());
        self
    }

    pub fn with_env(mut self, var_name: impl Display, value: impl Display) -> Self {
        self.config
            .env
            .get_or_insert_with(Default::default)
            .push(format!("{}={}", var_name, value));
        self
    }

    pub fn with_bind(mut self, from_local: impl Display, to_container: impl Display) -> Self {
        let host_config = self.config.host_config.get_or_insert_with(Default::default);
        host_config
            .binds
            .get_or_insert_with(Default::default)
            .push(format!("{}:{}", from_local, to_container));
        self
    }

    pub fn with_net(mut self, network_id: impl ToString) -> Self {
        let host_config = self.config.host_config.get_or_insert_with(Default::default);
        host_config.network_mode = Some(network_id.to_string());
        self
    }

    /// Whether [crate::ContainerNetwork] will wait for my execution
    /// before it removes all of its inner containers
    pub fn with_wait(mut self, wait: bool) -> Self {
        self.is_waited = wait;
        self
    }

    pub fn with_bind_current_exe_dir(self, to_container: impl Display) -> Self {
        let exe = env::current_exe().unwrap();
        println!(
            "binding {} --> {}",
            exe.parent().unwrap().to_string_lossy(),
            to_container
        );
        self.with_bind(exe.parent().unwrap().to_string_lossy(), to_container)
    }

    const CONTAINER_BOOTSTRAP_DIR: &'static str = "/tmp/target";
    pub fn with_bootstrap(self) -> Self {
        let exe = Path::new(Self::CONTAINER_BOOTSTRAP_DIR)
            .join(env::current_exe().unwrap().file_name().unwrap());

        let args = env::args();

        self.with_bind_current_exe_dir(Self::CONTAINER_BOOTSTRAP_DIR)
            .with_cmd([exe.to_string_lossy().into_owned()].into_iter().chain(args))
    }

    pub async fn build<'b>(mut self, docker: &Docker) -> Result<Container, Error>
    where
        T: Into<Cow<'b, str>>,
    {
        let name = self.opts.name.to_string();
        self.config.image = Some(self.image.build(docker).await?.id);
        let info = docker
            .create_container(Some(self.opts), self.config)
            .await?;
        Ok(Container::new(info.id, name, self.is_waited))
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct Container {
    id: String,
    name: String,

    /// Used for [crate::ContainerNetwork] to decide whether
    /// to wait for my execution before it removes all of its
    /// inner containers
    pub(crate) is_waited: bool,
}

impl Container {
    fn new(id: String, name: String, is_waited: bool) -> Self {
        Self {
            id,
            name,
            is_waited,
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub async fn start(&self, docker: &Docker) -> Result<(), Error> {
        docker.start_container::<String>(&self.id, None).await?;
        Ok(())
    }

    pub async fn rm(&self, docker: &Docker) -> Result<(), Error> {
        docker
            .remove_container(
                &self.id,
                Some(RemoveContainerOptions {
                    force: true,
                    ..Default::default()
                }),
            )
            .await?;
        Ok(())
    }

    pub async fn run(&self, docker: &Docker) -> Result<(), Error> {
        let logs = docker.logs::<String>(
            &self.id,
            Some(LogsOptions {
                follow: true,
                stdout: true,
                stderr: true,
                ..Default::default()
            }),
        );

        try {
            self.start(docker).await?;
            logs.try_for_each(|x| async move {
                use bollard::container::LogOutput::*;
                let prompt = self.name.to_string() + ":";
                match x {
                    StdErr { .. } => eprint!("{:<20} {}", prompt.blue(), x),
                    _ => print!("{:<20} {}", prompt.blue(), x),
                }
                Ok(())
            })
            .await?;
        }
    }

    pub fn log(
        &self,
        docker: &Docker,
        follow: bool,
    ) -> impl Stream<Item = Result<LogOutput, Error>> {
        docker
            .logs::<String>(
                &self.id,
                Some(LogsOptions {
                    follow,
                    stdout: true,
                    stderr: true,
                    ..Default::default()
                }),
            )
            .map_err(Error::from)
    }

    pub async fn wait(&self, docker: &Docker) -> Result<(), Error> {
        debug_assert!(self.is_waited);
        docker
            .wait_container(
                &self.name,
                Some(WaitContainerOptions {
                    condition: "not-running",
                }),
            )
            .map_err(|e| match e {
                bollard::errors::Error::DockerContainerWaitError { error, code } => eyre!(
                    "container {} exited with code {} {}",
                    self.name,
                    code,
                    error
                ),
                e => Error::from(e),
            })
            .map_ok(|_| ())
            .try_collect::<()>()
            .await?;
        Ok(())
    }

    pub async fn exec(
        &self,
        docker: &Docker,
        cmd: Option<impl IntoIterator<Item = impl ToString>>,
    ) -> Result<(), Error> {
        // see offical example from
        // https://github.com/fussybeaver/bollard/blob/31868e5186b7f4f24a9e6903945162b40f3ccea1/examples/exec.rs
        let exec = docker
            .create_exec(
                &self.id,
                CreateExecOptions {
                    attach_stdout: Some(true),
                    attach_stderr: Some(true),
                    cmd: cmd.map(|cmd| cmd.into_iter().map(|x| x.to_string()).collect()),
                    ..Default::default()
                },
            )
            .await?
            .id;
        if let StartExecResults::Attached { output, .. } = docker.start_exec(&exec, None).await? {
            output
                .try_for_each(|x| {
                    println!("{x}");
                    futures::future::ready(Ok(()))
                })
                .await?;
            Ok(())
        } else {
            unreachable!();
        }
    }
}
