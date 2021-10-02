use crate::utils::{load_yaml, get_value_mapping};
use std::{collections::BTreeMap, path::PathBuf};

#[derive(Default, Debug)]
pub struct Manifest {
    pub task_id: String,
    pub output_only: bool,
    pub time_limit: Option<f64>,
    pub memory_limit: Option<u64>,
    pub limit: Option<BTreeMap<String, (f64, u64)>>,
    pub compile_files: Option<BTreeMap<String, Vec<String>>>,
    pub checker: Option<String>,
    pub grouper: Option<String>,
    pub groups: Vec<(u64, u64)>,
}

impl Manifest {
    pub fn from(path: PathBuf) -> Self {
        let yaml = load_yaml(path);
        Manifest {
            task_id: get_value_mapping(&yaml, "task_id").as_str().unwrap().to_owned(),
            output_only: get_value_mapping(&yaml, "output_only").as_bool().unwrap_or(false),
            time_limit: get_value_mapping(&yaml, "time_limit").as_f64(),
            memory_limit: get_value_mapping(&yaml, "memory_limit").as_i64().map(|limit| limit as u64),
            limit: get_value_mapping(&yaml, "limit").as_mapping().map(|limits| {
                limits
                    .iter()
                    .map(|(language, limit)| {
                        (
                            language.as_str().unwrap().to_owned(),
                            (
                                limit["time_limit"].as_f64().unwrap(),
                                limit["memory_limit"].as_i64().unwrap() as u64,
                            ),
                        )
                    })
                    .collect()
            }),
            compile_files: get_value_mapping(&yaml, "compile_files").as_mapping().map(|compile_files| {
                compile_files
                    .iter()
                    .map(|(language, files)| {
                        (
                            language.as_str().unwrap().to_owned(),
                            files
                                .as_sequence()
                                .unwrap()
                                .iter()
                                .map(|file| file.as_str().unwrap().to_owned())
                                .collect(),
                        )
                    })
                    .collect()
            }),
            checker: get_value_mapping(&yaml, "checker").as_str().map(|checker| checker.to_owned()),
            grouper: get_value_mapping(&yaml, "grouper").as_str().map(|grouper| grouper.to_owned()),
            groups: get_value_mapping(&yaml, "groups")
                .as_sequence()
                .map(|groups| {
                    groups
                        .iter()
                        .map(|group| {
                            (
                                group["full_score"].as_i64().unwrap() as u64,
                                group["tests"].as_i64().unwrap() as u64,
                            )
                        })
                        .collect()
                })
                .unwrap(),
        }
    }
}
