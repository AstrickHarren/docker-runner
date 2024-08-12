use bollard::Docker;
use color_eyre::owo_colors::OwoColorize;
use docker_bootstrapper::{BootstrapDockerNet, ImageBuilder};
use dockerfiles::{DockerFile, From};

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;
    let df = DockerFile::new(From::image("alpine"));
    let img = ImageBuilder::new(&df);

    let hello = img
        .to_container("greeter")
        .with_wait(true)
        .start_with(async {
            println!("{}", "hello, world".green().bold());
        });

    BootstrapDockerNet::new("hello_exmpale", [hello])
        .run(Docker::connect_with_defaults)
        .await
}
