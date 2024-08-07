use bollard::Docker;
use docker_bootstrapper::ImageBuilder;

#[tokio::test]
async fn test_container() -> color_eyre::Result<()> {
    color_eyre::install()?;
    let docker = Docker::connect_with_defaults()?;
    let dockerfile = {
        use dockerfiles::*;
        DockerFile::new(From::image("alpine"))
    };

    let img = ImageBuilder::new(&dockerfile).build(&docker).await?;
    let container = img
        .into_container_builder("my_container")
        .with_cmd("ls -al /".split(" "))
        .build(&docker)
        .await?;

    container.run(&docker).await?;

    Ok(())
}
