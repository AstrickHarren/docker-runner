use std::time::Duration;

use bollard::Docker;
use color_eyre::owo_colors::OwoColorize;
use docker_bootstrapper::{BootstrapDockerNet, ImageBuilder};
use dockerfiles::*;
use tokio::time::sleep;

#[tokio::test]
async fn bare_bootstrap() -> color_eyre::Result<()> {
    color_eyre::install()?;
    let docker = Docker::connect_with_defaults()?;
    let dockerfile = DockerFile::new(From::image("ubuntu"));

    let img = ImageBuilder::new(&dockerfile);
    let container = img
        .to_container("test_bootstrap")
        .with_bootstrap()
        .build(&docker)
        .await?;

    container
        .run(&docker)
        .await
        .and(container.rm(&docker).await)?;

    Ok(())
}

#[tokio::test]
async fn bootstrapper() -> color_eyre::Result<()> {
    color_eyre::install()?;
    let dockerfile = DockerFile::new(From::image("alpine"));
    let img = ImageBuilder::new(&dockerfile);

    let d1 = img.to_container("d1").with_wait(true).start();
    let d2 = img.to_container("d2").with_wait(true).start();
    let d3 = img
        .to_container("d3")
        .with_wait(true)
        .start()
        .then(|_| async {
            println!("{}", "I am docker 3, sleeping for 1 sec...".blue());
            sleep(Duration::from_secs(1)).await;
            println!("{}", "sleep complete".blue());
        });

    let d1 = d1.then(|_| async {
        println!("{}", "I am docker 1".green());
    });
    let d2 = d2.then(|_| async {
        println!("{}", "I am docker 2".yellow());
    });

    BootstrapDockerNet::new("bootstrapper", [d1, d2, d3])
        .run(Docker::connect_with_defaults)
        .await
}

/// This is difficult to test with boostrapping. For example,
/// it is not correct simply to assert the result here to be
/// an error. Because not every container in the network is
/// going to produce an error by this function.
#[ignore = "panics with bootstraps"]
#[tokio::test]
async fn bootstrapper_panic() -> color_eyre::Result<()> {
    color_eyre::install()?;
    let dockerfile = DockerFile::new(From::image("alpine"));
    let img = ImageBuilder::new(&dockerfile);

    let d1 = img.to_container("d1").with_wait(true).start();
    let d2 = img
        .to_container("d2")
        .with_wait(true)
        .start()
        .then(|_| async {
            println!("{}", "I am docker 2, sleeping for 5 sec...".yellow());
            sleep(Duration::from_secs(5)).await;
            println!("{}", "sleep complete".blue());
        });
    let d3 = img
        .to_container("d3")
        .with_wait(true)
        .start()
        .then(|_| async {
            println!("{}", "I am docker 3, sleeping for 1 sec...".blue());
            sleep(Duration::from_secs(1)).await;
            println!("{}", "I'm going to panic now".red());
            panic!("test panic");
        });

    let d1 = d1.then(|_| async {
        println!("{}", "I am docker 1".green());
    });

    BootstrapDockerNet::new("bootstrapper", [d1, d2, d3])
        .run(Docker::connect_with_defaults)
        .await
}
