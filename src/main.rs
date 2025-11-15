use runr::{Config, Result, read_pipeline, repo_checkout};

fn main() {
    let config = Config::from_env();
    let exit_code = match run(&config) {
        Ok(true) => 0,
        Ok(false) => 1,
        Err(e) => {
            eprintln!("{e}");
            1
        }
    };
    config
        .cleanup()
        .expect("Unable to clean up repository directory");
    std::process::exit(exit_code)
}

fn run(config: &Config) -> Result<bool> {
    repo_checkout(config)?;
    let pipeline = read_pipeline(config)?;
    let run_config = config.run_config(&pipeline);
    let mut run = pipeline.run(run_config);

    run.start()?;
    let succ = run.is_completed() && run.is_succeeded();
    run.cleanup()?;
    Ok(succ)
}
