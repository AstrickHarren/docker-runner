use color_eyre::eyre::Error;
use std::{cell::RefCell, io::Write, sync::Arc};

use bollard::{image::BuildImageOptions, Docker};
use dockerfiles::DockerFile;
use futures::{future::ready, lock::Mutex, TryStreamExt};

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

#[derive(Debug, Clone)]
pub struct Image {
    pub id: String,
}

impl Image {
    pub fn new(id: String) -> Self {
        Self { id }
    }
}
