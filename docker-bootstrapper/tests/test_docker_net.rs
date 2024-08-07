use bollard::Docker;
use docker_bootstrapper::{ContainerBuilder, ContainerNetworkBuilder, Image, ImageBuilder};
use dockerfiles::{DockerFile, From};

#[tokio::test]
async fn test_docker_net() -> color_eyre::Result<()> {
    color_eyre::install()?;
    let docker = Docker::connect_with_defaults()?;

    //let p1 = container_builder(&docker, "p1").await?;
    //let p2 = container_builder(&docker, "p2").await?;
    //let p3 = container_builder(&docker, "p3").await?;
    let yes = container_builder(&docker, "p4")
        .await?
        .with_cmd(["yes"])
        .with_wait(true);
    let network = ContainerNetworkBuilder::new("test").with_containers([yes]);

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
        .with_cmd("ls -al /".split(" ")))
}
