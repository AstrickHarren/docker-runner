use std::{env, fmt::Display};

use bollard::{
    container::{Config, CreateContainerOptions, LogsOptions, RemoveContainerOptions},
    errors::Error,
    exec::{CreateExecOptions, StartExecResults},
    Docker,
};

use color_eyre::owo_colors::OwoColorize;

use futures::{TryFutureExt, TryStreamExt};

use crate::Image;

impl Image {
    pub fn into_container_builder(self, name: &str) -> ContainerBuilder {
        ContainerBuilder::new(name, self)
    }
}

pub struct ContainerBuilder<'a> {
    opts: CreateContainerOptions<&'a str>,
    config: Config<String>,
}

impl<'a> ContainerBuilder<'a> {
    pub fn new(name: &'a str, image: Image) -> Self {
        Self {
            opts: CreateContainerOptions {
                name,
                ..Default::default()
            },
            config: Config {
                image: Some(image.id),
                tty: Some(true),
                attach_stdout: Some(true),
                attach_stderr: Some(true),
                ..Default::default()
            },
        }
    }

    pub fn with_cmd(mut self, cmd: impl IntoIterator<Item = impl ToString>) -> Self {
        self.config.cmd = Some(cmd.into_iter().map(|x| x.to_string()).collect());
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

    pub fn with_bind_current_exe_dir(self, to_container: impl Display) -> Self {
        let exe = env::current_exe().unwrap();
        self.with_bind(exe.parent().unwrap().to_string_lossy(), to_container)
    }

    pub async fn build(self, docker: &Docker) -> Result<Container, Error> {
        let name = self.opts.name.to_string();
        let info = docker
            .create_container(Some(self.opts), self.config)
            .await?;
        Ok(Container::new(info.id, name))
    }
}

#[derive(Debug)]
pub struct Container {
    id: String,
    name: String,
}

impl Container {
    fn new(id: String, name: String) -> Self {
        Self { id, name }
    }

    async fn start(&self, docker: &Docker) -> Result<(), Error> {
        docker.start_container::<String>(&self.id, None).await?;
        Ok(())
    }

    async fn rm(&self, docker: &Docker) -> Result<(), Error> {
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
                stdout: true,
                stderr: true,
                ..Default::default()
            }),
        );

        let start = self.start(docker);
        let task = logs.try_for_each(|x| async move {
            use bollard::container::LogOutput::*;
            let prompt = self.name.to_string() + ":";
            match x {
                StdErr { .. } => eprint!("{:<20} {}", prompt.blue(), x),
                _ => print!("{:<20} {}", prompt.blue(), x),
            }
            Ok(())
        });
        let rm = self.rm(docker);

        // if start succeed, do task, but always do rm
        start.and_then(|_| task).await.and(rm.await)
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
