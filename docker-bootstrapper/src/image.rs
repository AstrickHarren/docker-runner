use color_eyre::eyre::Error;
use std::{borrow::Cow, io::Write};

use bollard::{image::BuildImageOptions, Docker};
use futures::{future::ready, TryStreamExt};

#[derive(Clone, Copy)]
pub struct ImageBuilder<T> {
    docker_file: T,
}

impl<T> ImageBuilder<T> {
    pub fn new(docker_file: T) -> Self {
        Self { docker_file }
    }

    pub async fn build<'a>(self, docker: &Docker) -> Result<Image, Error>
    where
        T: Into<Cow<'a, str>>,
    {
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

    fn create_docker_tarball<'a>(self) -> Vec<u8>
    where
        T: Into<Cow<'a, str>>,
    {
        let mut header = tar::Header::new_gnu();
        let dockerfile: Cow<_> = self.docker_file.into();

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
