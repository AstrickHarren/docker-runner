use bollard::Docker;
use docker_bootstrapper::ImageBuilder;
use dockerfiles::*;

#[tokio::test]
async fn test_container_ls() -> color_eyre::Result<()> {
    color_eyre::install()?;
    let docker = Docker::connect_with_defaults()?;
    let dockerfile = DockerFile::new(From::image("alpine"));
    let container = ImageBuilder::new(&dockerfile)
        .build(&docker)
        .await?
        .into_container_builder("my_container")
        .with_cmd("ls -al /".split(" "))
        .build(&docker)
        .await?;
    container
        .run(&docker)
        .await
        .and(container.rm(&docker).await)?;

    Ok(())
}

#[tokio::test]
async fn test_container_postgres() -> color_eyre::Result<()> {
    color_eyre::install()?;
    let docker = Docker::connect_with_defaults()?;
    let container = ImageBuilder::new(&DockerFile::new(From::image("postgres")))
        .build(&docker)
        .await?
        .into_container_builder("postgres")
        .with_env("POSTGRES_PASSWORD", "postgres")
        .build(&docker)
        .await?;
    container
        .run(&docker)
        .await
        .and(container.rm(&docker).await)?;

    Ok(())
}
