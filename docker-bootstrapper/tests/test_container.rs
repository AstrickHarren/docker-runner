use bollard::Docker;
use docker_bootstrapper::ImageBuilder;
use dockerfiles::*;

#[tokio::test]
async fn test_container_short() -> color_eyre::Result<()> {
    color_eyre::install()?;
    let docker = Docker::connect_with_defaults()?;
    let dockerfile = DockerFile::new(From::image("alpine"));

    ImageBuilder::new(&dockerfile)
        .build(&docker)
        .await?
        .into_container_builder("my_container")
        .with_cmd("ls -al /".split(" "))
        .build(&docker)
        .await?
        .run(&docker)
        .await?;

    Ok(())
}

#[tokio::test]
async fn test_container_long() -> color_eyre::Result<()> {
    color_eyre::install()?;
    let docker = Docker::connect_with_defaults()?;
    ImageBuilder::new(&DockerFile::new(From::image("postgres")))
        .build(&docker)
        .await?
        .into_container_builder("postgres")
        .with_env("POSTGRES_PASSWORD", "postgres")
        .build(&docker)
        .await?
        .run(&docker)
        .await?;

    Ok(())
}
