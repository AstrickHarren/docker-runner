use bollard::Docker;
use docker_bootstrapper::{ContainerBuilder, ContainerNetworkBuilder, ImageBuilder};
use dockerfiles::{DockerFile, From};

#[tokio::test]
async fn no_wait() -> color_eyre::Result<()> {
    color_eyre::install()?;
    let docker = Docker::connect_with_defaults()?;

    let p1 = container_builder(&docker, "p1").await?;
    let p2 = container_builder(&docker, "p2").await?;
    let p3 = container_builder(&docker, "p3").await?;
    let postgres = ImageBuilder::new(&DockerFile::new(From::image("postgres")))
        .build(&docker)
        .await?
        .into_container_builder("postgres")
        .with_wait(false)
        .with_env("POSTGRES_PASSWORD", "postgres");
    let network = ContainerNetworkBuilder::new("test").with_containers([p1, p2, p3, postgres]);
    network.build(&docker).await?.run(&docker).await?;
    Ok(())
}

#[tokio::test]
async fn wait() -> color_eyre::Result<()> {
    color_eyre::install()?;
    let docker = Docker::connect_with_defaults()?;

    let p1 = container_builder(&docker, "p1").await?;
    let p2 = container_builder(&docker, "p2").await?;
    let p3 = container_builder(&docker, "p3").await?;
    let postgres = ImageBuilder::new(&DockerFile::new(From::image("postgres")))
        .build(&docker)
        .await?
        .into_container_builder("postgres")
        .with_wait(true)
        .with_env("POSTGRES_PASSWORD", "postgres");
    let network = ContainerNetworkBuilder::new("test").with_containers([p1, p2, p3, postgres]);
    network.build(&docker).await?.run(&docker).await?;
    Ok(())
}

#[tokio::test]
async fn double_postgres() -> color_eyre::Result<()> {
    color_eyre::install()?;
    let docker = Docker::connect_with_defaults()?;

    let p1 = ImageBuilder::new(&DockerFile::new(From::image("postgres")))
        .build(&docker)
        .await?
        .into_container_builder("p1")
        .with_wait(true)
        .with_env("POSTGRES_PASSWORD", "postgres");
    let p2 = ImageBuilder::new(&DockerFile::new(From::image("postgres")))
        .build(&docker)
        .await?
        .into_container_builder("p2")
        .with_wait(true)
        .with_env("POSTGRES_PASSWORD", "postgres");
    let network = ContainerNetworkBuilder::new("test").with_containers([p1, p2]);
    network.build(&docker).await?.run(&docker).await?;
    Ok(())
}

#[tokio::test]
async fn postgres_delay() -> color_eyre::Result<()> {
    color_eyre::install()?;
    let docker = Docker::connect_with_defaults()?;

    let p1 = ImageBuilder::new(&DockerFile::new(From::image("postgres")))
        .build(&docker)
        .await?
        .into_container_builder("p1")
        .with_wait(true)
        .with_env("POSTGRES_PASSWORD", "postgres");
    let delay = container_builder(&docker, "yes")
        .await?
        .with_cmd(["sh", "-c", "sleep 5; echo hello"])
        .with_wait(true);
    let network = ContainerNetworkBuilder::new("test").with_containers([p1, delay]);
    network.build(&docker).await?.run(&docker).await?;
    Ok(())
}
async fn container_builder<'a>(
    docker: &Docker,
    name: &'a str,
) -> color_eyre::Result<ContainerBuilder<'a>> {
    let docker_file = DockerFile::new(From::image("alpine"));
    let img_builder = ImageBuilder::new(&docker_file).build(docker).await?;
    Ok(img_builder
        .into_container_builder(name)
        .with_wait(true)
        .with_cmd("ls -al /".split(" ")))
}
