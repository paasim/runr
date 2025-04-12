use runr::{Config, Result, read_pipeline, repo_checkout};

fn main() {
    let config = Config::from_env();
    if let Err(e) = run(&config) {
        eprintln!("{e}")
    };
    config
        .cleanup()
        .expect("Unable to clean up repository directory")
}

fn run(config: &Config) -> Result<()> {
    repo_checkout(config)?;
    let pipeline = read_pipeline(config)?;
    let run_config = config.run_config(&pipeline);
    let mut run = pipeline.run(run_config);

    run.start()?;
    run.cleanup()?;
    Ok(())
}
