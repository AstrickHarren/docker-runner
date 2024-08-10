use std::time::Duration;

use bollard::Docker;
use docker_bootstrapper::{BootstrapDockerNet, ImageBuilder};
use dockerfiles::*;
use tokio::time::sleep;

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

#[tokio::test]
async fn bootstrapper() -> color_eyre::Result<()> {
    color_eyre::install()?;
    let dockerfile = DockerFile::new(From::image("alpine"));
    let img = ImageBuilder::new(&dockerfile);

    let d1 = img.into_container_builder("d1").with_wait(true).perform();
    let d2 = img.into_container_builder("d2").with_wait(true).perform();
    let d3 = img
        .into_container_builder("d3")
        .with_wait(true)
        .perform()
        .then(|_| async {
            println!("I am docker 3, sleeping for 10 secs...");
            sleep(Duration::from_secs(10)).await;
        });

    let d1 = d1.then(|_| async {
        println!("I am docker 1");
    });
    let d2 = d2.then(|_| async {
        println!("I am docker 2");
    });

    BootstrapDockerNet::new("boot", [d1, d2, d3])
        .run(Docker::connect_with_defaults)
        .await
}
