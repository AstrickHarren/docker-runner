use std::io::Write;

use bollard::{
    container::{Config, CreateContainerOptions, RemoveContainerOptions},
    errors::Error,
    exec::{CreateExecOptions, StartExecResults},
    image::BuildImageOptions,
    Docker,
};
use dockerfiles::DockerFile;
use futures::{future::ready, StreamExt, TryFutureExt, TryStreamExt};

pub struct ImageBuilder<'a> {
    docker_file: &'a DockerFile,
}

impl<'a> ImageBuilder<'a> {
    pub fn new(docker_file: &'a DockerFile) -> Self {
        Self { docker_file }
    }

    pub async fn build(self, docker: &Docker) -> Result<Image, Error> {
        let opts = BuildImageOptions {
            dockerfile: "Dockerfile",
            ..Default::default()
        };

        let tar = self.create_docker_tarball().into();
        let images = docker.build_image(opts, None, Some(tar));
        let infos = images
            .inspect_ok(|x| {
                // TODO: use tracing
                x.stream.as_ref().inspect(|x| print!("{}", x));
            })
            .try_filter_map(|x| ready(Ok(x.aux)));

        // TODO: stop using vec
        let id: Vec<_> = infos.try_collect().await?;
        let id = id
            .into_iter()
            .next()
            .unwrap()
            .id
            .expect("image built without id");
        Ok(Image::new(id))
    }

    fn create_docker_tarball(&self) -> Vec<u8> {
        let mut header = tar::Header::new_gnu();
        let dockerfile = self.docker_file.to_string();

        header.set_path("Dockerfile").unwrap();
        header.set_size(dockerfile.len() as u64);
        header.set_mode(0o755);
        header.set_cksum();
        let mut tar = tar::Builder::new(Vec::new());
        tar.append(&header, dockerfile.as_bytes()).unwrap();

        let uncompressed = tar.into_inner().unwrap();
        let mut c = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
        c.write_all(&uncompressed).unwrap();
        c.finish().unwrap()
    }
}

#[derive(Debug)]
pub struct Image {
    pub id: String,
}

impl Image {
    fn new(id: String) -> Self {
        Self { id }
    }

    pub async fn create_container(self, name: &str, docker: &Docker) -> Result<Container, Error> {
        let opts = CreateContainerOptions {
            name,
            platform: Some("linux/amd64"), // TODO: support multi-platform
        };

        let config = Config {
            image: Some(self.id),
            tty: Some(true),
            ..Default::default()
        };

        let info = docker.create_container(Some(opts), config).await?;
        Ok(Container::new(info.id, name.to_string()))
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
        docker.start_container::<String>(&self.id, None).await
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
            .await
    }

    pub async fn run(
        &self,
        docker: &Docker,
        cmd: Option<impl IntoIterator<Item = impl ToString>>,
    ) -> Result<(), Error> {
        self.start(docker).await?;
        let exec = self.exec(docker, cmd).await;
        self.rm(docker).await.and(exec)?;
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
                .await
        } else {
            unreachable!();
        }
    }
}
