use bollard::Docker;
use docker_bootstrapper::ImageBuilder;

#[tokio::test]
async fn test_docker() -> color_eyre::Result<()> {
    color_eyre::install()?;
    let docker = Docker::connect_with_defaults()?;
    let dockerfile = {
        use dockerfiles::*;
        DockerFile::new(From::image("alpine"))
    };

    let img = ImageBuilder::new(&dockerfile).build(&docker).await?;
    let container = img.create_container("my_container", &docker).await?;
    container.run(&docker, Some(["ls", "-l", "/"])).await?;

    Ok(())
}
