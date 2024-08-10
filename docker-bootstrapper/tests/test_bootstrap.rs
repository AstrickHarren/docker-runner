use std::{env, path::Path};

use bollard::Docker;
use docker_bootstrapper::ImageBuilder;

#[tokio::test]
async fn bootstrap() -> color_eyre::Result<()> {
    color_eyre::install()?;
    let docker = Docker::connect_with_defaults()?;
    let dockerfile = {
        use dockerfiles::*;
        DockerFile::new(From::image("ubuntu"))
    };

    let img = ImageBuilder::new(&dockerfile).build(&docker).await?;

    let tmp_dir: &Path = "/tmp/target".as_ref();
    let exe_dir = tmp_dir.join(env::current_exe().unwrap().file_name().unwrap());
    let cmd = format!("{}", exe_dir.to_string_lossy());

    let container = img
        .into_container_builder("test_bootstrap")
        .with_bootstrap()
        .build(&docker)
        .await?;

    container.run(&docker).await?;

    Ok(())
}
