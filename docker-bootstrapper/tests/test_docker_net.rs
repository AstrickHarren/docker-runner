use bollard::Docker;
use docker_bootstrapper::{ContainerNetworkBuilder, ImageBuilder};
use dockerfiles::{DockerFile, From};

#[tokio::test]
async fn net_no_wait() -> color_eyre::Result<()> {
    color_eyre::install()?;
    let docker = Docker::connect_with_defaults()?;
    let alpine = DockerFile::new(From::image("alpine"));

    let p = |name| {
        ImageBuilder::new(&alpine)
            .into_container_builder(name)
            .with_cmd("ls -al /".split(" "))
            .with_wait(true)
    };

    let p1 = p("p1");
    let p2 = p("p2");
    let p3 = p("p3");
    let df = DockerFile::new(From::image("postgres"));
    let postgres = ImageBuilder::new(&df)
        .into_container_builder("postgres")
        .with_wait(false)
        .with_env("POSTGRES_PASSWORD", "postgres");
    let network = ContainerNetworkBuilder::new("test").with_containers([p1, p2, p3, postgres]);
    network.build(&docker).await?.run(&docker).await?;
    Ok(())
}

#[ignore = "needs wait"]
#[tokio::test]
async fn net_wait() -> color_eyre::Result<()> {
    color_eyre::install()?;
    let docker = Docker::connect_with_defaults()?;
    let alpine = DockerFile::new(From::image("alpine"));

    let p = |name| {
        ImageBuilder::new(&alpine)
            .into_container_builder(name)
            .with_cmd("ls -al /".split(" "))
            .with_wait(true)
    };

    let p1 = p("p1");
    let p2 = p("p2");
    let p3 = p("p3");
    let df = DockerFile::new(From::image("postgres"));
    let postgres = ImageBuilder::new(&df)
        .into_container_builder("postgres")
        .with_wait(true)
        .with_env("POSTGRES_PASSWORD", "postgres");
    let network = ContainerNetworkBuilder::new("test").with_containers([p1, p2, p3, postgres]);
    network.build(&docker).await?.run(&docker).await?;
    Ok(())
}

#[ignore = "needs wait"]
#[tokio::test]
async fn net_double_postgres() -> color_eyre::Result<()> {
    color_eyre::install()?;
    let docker = Docker::connect_with_defaults()?;
    let postgres = DockerFile::new(From::image("postgres"));

    let p1 = ImageBuilder::new(&postgres)
        .into_container_builder("p1")
        .with_wait(true)
        .with_env("POSTGRES_PASSWORD", "postgres");
    let p2 = ImageBuilder::new(&postgres)
        .into_container_builder("p2")
        .with_wait(true)
        .with_env("POSTGRES_PASSWORD", "postgres");
    let network = ContainerNetworkBuilder::new("test").with_containers([p1, p2]);
    network.build(&docker).await?.run(&docker).await?;
    Ok(())
}

#[ignore = "needs wait"]
#[tokio::test]
async fn net_postgres_delay() -> color_eyre::Result<()> {
    color_eyre::install()?;
    let docker = Docker::connect_with_defaults()?;
    let postgres = DockerFile::new(From::image("postgres"));
    let alpine = DockerFile::new(From::image("alpine"));

    let p1 = ImageBuilder::new(&postgres)
        .into_container_builder("p1")
        .with_wait(true)
        .with_env("POSTGRES_PASSWORD", "postgres");
    let delay = ImageBuilder::new(&alpine)
        .into_container_builder("delay")
        .with_cmd(["sh", "-c", "sleep 5; echo hello"])
        .with_wait(true);
    let network = ContainerNetworkBuilder::new("test").with_containers([p1, delay]);
    network.build(&docker).await?.run(&docker).await?;
    Ok(())
}
