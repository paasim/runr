use runr::{Config, Pipeline, repo_checkout};

const DEFAULT_IMAGE: &str = "docker.io/library/debian:latest";

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
    let pipeline = Pipeline::read_from(&mut yaml.as_bytes(), &None).unwrap();
    let config = Config::from_env();
    repo_checkout(&config).unwrap();
    let run_config = config.run_config(&pipeline);
    let mut run = pipeline.run(run_config);
    run.start().unwrap();
    assert!(run.is_completed());
    assert!(run.cleanup().unwrap() == 0);
    config.cleanup().unwrap();
}

#[ignore]
#[test]
fn simple_containerized_workflow_succeeds() {
    let yaml = format!(
        r#"
        default_image: {DEFAULT_IMAGE}
        tasks:
        - commands: |
            echo starting step 1a
            echo ending step 1a
          name: step-1a
        - commands: |
            echo starting step 2
            echo ending step 2
          name: step-2
          image: docker.io/library/ubuntu:latest
        - commands: |
            python -c 'print(1)'
          name: step-1b
          depends: ["step-1a"]
          image: docker.io/library/python:latest
        "#
    );
    let pipeline =
        Pipeline::read_from(&mut yaml.as_bytes(), &Some(DEFAULT_IMAGE.to_string())).unwrap();
    let config = Config::from_env();
    repo_checkout(&config).unwrap();
    let run_config = config.run_config(&pipeline);
    let mut run = pipeline.run(run_config);
    run.start().unwrap();
    assert!(run.is_completed());
    assert!(run.cleanup().unwrap() == 0);
    config.cleanup().unwrap();
}

#[ignore]
#[test]
fn fails_work_as_expected() {
    let yaml = r#"
        n_parallel: 0
        tasks:
        - commands: |
            echo ensuring debian is pulled
          name: step-0
          image: docker.io/library/debian:latest
        - commands: |
            echo starting step 1a
            sleep 1
            echo eror >&2
            exit 42
            echo ending step 1a
          name: step-1a
          depends: ["step-0"]
        - commands: |
            echo starting step 2
            echo which will be aborted as step 1a will fail
            sleep 5
            echo ending step 2
          name: step-2
          image: docker.io/library/debian:latest
          depends: ["step-0"]
        - commands: |
            echo starting step 1b
            sleep 1
            echo ending step 1b
          name: step-1b
          depends: ["step-1a"]"#;
    let pipeline = Pipeline::read_from(&mut yaml.as_bytes(), &None).unwrap();
    let config = Config::from_env();
    repo_checkout(&config).unwrap();
    let run_config = config.run_config(&pipeline);
    let mut run = pipeline.run(run_config);
    run.start().unwrap();
    assert!(!run.is_completed());
    assert!(run.cleanup().unwrap() > 0);
    config.cleanup().unwrap();
}

#[ignore]
#[test]
fn last_step_failing_does_not_result_into_completed() {
    let yaml = r#"
        tasks:
        - commands: |
            exit 0
          name: step-1a
        - commands: |
            exit 1
          name: step-1b
          depends: ["step-1a"]
          "#;
    let pipeline = Pipeline::read_from(&mut yaml.as_bytes(), &None).unwrap();
    let config = Config::from_env();
    repo_checkout(&config).unwrap();
    let run_config = config.run_config(&pipeline);
    let mut run = pipeline.run(run_config);
    run.start().unwrap();
    config.cleanup().unwrap();
    assert!(run.is_completed());
    assert!(!run.is_succeeded());
    assert!(run.cleanup().unwrap() == 0);
}
