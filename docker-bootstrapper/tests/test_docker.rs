use bollard::Docker;

#[tokio::test]
async fn test_docker() -> color_eyre::Result<()> {
    color_eyre::install()?;
    let docker = Docker::connect_with_defaults()?;
    let ver = docker.version().await?;

    println!("version: {:?}", ver);

    Ok(())
}
