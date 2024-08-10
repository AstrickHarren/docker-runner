use std::{env, path::Path};

use bollard::Docker;
use docker_bootstrapper::{BootstrapDockerNet, ImageBuilder};
use dockerfiles::*;

#[tokio::test]
async fn bootstrap() -> color_eyre::Result<()> {
    color_eyre::install()?;
    let docker = Docker::connect_with_defaults()?;
    let dockerfile = DockerFile::new(From::image("ubuntu"));

    let img = ImageBuilder::new(&dockerfile);
    let container = img
        .into_container_builder("test_bootstrap")
        .with_bootstrap()
        .build(&docker)
        .await?;

    container.run(&docker).await?;

    Ok(())
}
