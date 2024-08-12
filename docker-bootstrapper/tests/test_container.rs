use bollard::Docker;
use docker_bootstrapper::ImageBuilder;
use dockerfiles::*;

#[tokio::test]
async fn container_ls() -> color_eyre::Result<()> {
    color_eyre::install()?;
    let docker = Docker::connect_with_defaults()?;
    let dockerfile = DockerFile::new(From::image("alpine"));
    let container = ImageBuilder::new(&dockerfile)
        .to_container("test_container_ls")
        .with_cmd("ls -al /".split(" "))
        .build(&docker)
        .await?;
    container
        .run(&docker)
        .await
        .and(container.rm(&docker).await)?;

    Ok(())
}

#[ignore = "needs wait"]
#[tokio::test]
async fn container_postgres() -> color_eyre::Result<()> {
    color_eyre::install()?;
    let docker = Docker::connect_with_defaults()?;
    let container = ImageBuilder::new(&DockerFile::new(From::image("postgres")))
        .to_container("postgres")
        .with_env("POSTGRES_PASSWORD", "postgres")
        .build(&docker)
        .await?;
    container
        .run(&docker)
        .await
        .and(container.rm(&docker).await)?;

    Ok(())
}
