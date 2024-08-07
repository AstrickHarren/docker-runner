use docker_bootstrapper::ContainerNetworkBuilder;

#[tokio::test]
async fn test_docker_net() -> color_eyre::Result<()> {
    color_eyre::install()?;
    let network = ContainerNetworkBuilder::new("test");
    Ok(())
}
