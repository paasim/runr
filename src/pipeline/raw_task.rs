#[derive(Debug, serde::Deserialize, PartialEq)]
pub struct RawTask {
    pub name: String,
    pub commands: String,
    pub image: Option<String>,
    pub depends: Option<Vec<String>>,
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn parse_simple() {
        let task_yaml = r#"
            name: "test"
            image: image
            commands: echo cmd && echo moi
            depends: ["step0", "step1"]
        "#;
        let task: RawTask = serde_yaml::from_str(task_yaml).unwrap();
        let task_exp = RawTask {
            name: String::from("test"),
            commands: String::from("echo cmd && echo moi"),
            image: Some(String::from("image")),
            depends: Some(vec![String::from("step0"), String::from("step1")]),
        };
        assert_eq!(task, task_exp)
    }

    #[test]
    fn parse_literal_style() {
        let task_yaml = r#"
            name: this is a name
            commands: |
              cmd
              exit 0
        "#;
        let task: RawTask = serde_yaml::from_str(task_yaml).unwrap();
        let task_exp = RawTask {
            name: String::from("this is a name"),
            commands: String::from("cmd\nexit 0\n"),
            image: None,
            depends: None,
        };
        assert_eq!(task, task_exp)
    }
}
