use runr::{Config, Pipeline, repo_checkout};

const DEFAULT_IMAGE: &str = "docker.io/library/debian:bookworm";

#[ignore]
#[test]
fn simple_workflow_succeeds() {
    let yaml = r#"
        tasks:
        - commands: |
            echo starting step 1a
            echo ending step 1a
          name: step-1a
        - commands: |
            echo starting step 2
            echo ending step 2
          name: step-2
        - commands: |
            echo starting step 1b
            echo ending step 1b
          name: step-1b
          depends: ["step-1a"]"#;
    let pipeline = Pipeline::read_from(&mut yaml.as_bytes(), DEFAULT_IMAGE).unwrap();
    let config = Config::from_env();
    repo_checkout(&config).unwrap();
    let run_config = config.run_config(&pipeline);
    let mut run = pipeline.run(run_config);
    run.start().unwrap();
    assert!(run.is_completed());
    assert!(run.cleanup().unwrap() == 0);
}

#[ignore]
#[test]
fn fails_work_as_expected() {
    let yaml = r#"
        n_parallel: 0
        tasks:
        - commands: |
            echo starting step 1a
            sleep 1
            echo eror >&2
            exit 42
            echo ending step 1a
          name: step-1a
        - commands: |
            echo starting step 2
            sleep 5
            echo ending step 2
          name: step-2
        - commands: |
            echo starting step 1b
            sleep 1
            echo ending step 1b
          name: step-1b
          depends: ["step-1a"]"#;
    let pipeline = Pipeline::read_from(&mut yaml.as_bytes(), DEFAULT_IMAGE).unwrap();
    let config = Config::from_env();
    repo_checkout(&config).unwrap();
    let run_config = config.run_config(&pipeline);
    let mut run = pipeline.run(run_config);
    run.start().unwrap();
    assert!(!run.is_completed());
    assert!(run.cleanup().unwrap() > 0);
}
