macro_rules! s {
    ($x:expr) => {
        String::from($x);
    };
}
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::instance::error::InstanceError;

#[derive(Default, Debug)]
pub struct Instance {
    pub box_path: PathBuf,
    pub log_file: PathBuf,
    pub box_id: u32,
    pub bin_path: PathBuf,
    pub time_limit: f64,
    pub memory_limit: u64,
    pub input_path: PathBuf,
    pub output_path: PathBuf,
    pub runner_path: PathBuf,
}

pub enum RunVerdict {
    OK,
    TLE,
    MLE,
    RE,
    XX,
    OT,
}

pub struct InstanceResult {
    pub status: RunVerdict,
    pub time_usage: u32,
    pub memory_usage: u32,
}

pub fn get_env(name: &'static str) -> Result<String, InstanceError> {
    env::var(name)
        .map_err(|_| InstanceError::EnvironmentError(format!("cannot get {} from env", name)))
}

impl Instance {
    fn get_arguments(&self) -> Result<Vec<String>, InstanceError> {
        let mut args: Vec<String> = vec![
            s!("-b"),
            self.box_id.to_string(),
            s!("-M"),
            self.log_file.to_str().unwrap().to_string(),
            s!("-t"),
            self.time_limit.to_string(),
            s!("-w"),
            (self.time_limit + 5.0).to_string(),
            s!("-x"),
            (self.time_limit + 1.0).to_string(),
            s!("-i"),
            s!("input"),
            s!("-o"),
            s!("output"),
            s!("--run"),
            s!("--"),
            s!("runner"),
            s!("--cg"),
            s!("--cg-timing"),
            s!("--processes=128"),
            format!("--cg-mem={}", self.memory_limit),
        ];

        if Path::new(&get_env("ALTERNATIVE_PATH")?).is_dir() {
            args.push(format!("--dir={}", get_env("ALTERNATIVE_PATH")?));
        }

        Ok(args)
    }

    fn check_root_permission(&self) -> Result<(), InstanceError> {
        let permission_result = Command::new("id").arg("-u").output();
        match permission_result {
            Ok(output) => {
                let output_string = String::from_utf8(output.stdout).unwrap();
                if output_string.trim() == "0" {
                    Ok(())
                } else {
                    Err(InstanceError::PermissionError(s!(
                        "isolate must be run as root"
                    )))
                }
            }
            _ => Err(InstanceError::PermissionError(s!(
                "unable to get current user id"
            ))),
        }
    }

    pub fn init(&mut self) -> Result<(), InstanceError> {
        self.check_root_permission()?;

        let box_path = Command::new(get_env("ISOLATE_PATH")?)
            .args(&["--init", "--cg", "-b"])
            .arg(self.box_id.to_string())
            .output()
            .map_err(|_| {
                InstanceError::PermissionError(s!("Unable to run isolate --init command."))
            })?;

        self.box_path = PathBuf::from(
            String::from_utf8(box_path.stdout)
                .unwrap()
                .strip_suffix("\n")
                .unwrap(),
        )
        .join("box");
        
        let tmp_path = get_env("TEMPORARY_PATH")?;
        self.log_file = PathBuf::from(tmp_path).join(format!("tmp_log_{}.txt", self.box_id));

        fs::copy(
            self.input_path.to_str().unwrap(),
            self.box_path.join("input").to_str().unwrap(),
        )
        .map_err(|_| {
            InstanceError::PermissionError(s!("Unable to copy input file into box directory"))
        })?;

        fs::copy(
            self.bin_path.to_str().unwrap(),
            self.box_path.join(self.bin_path.file_name().unwrap()).to_str().unwrap(),
        )
        .map_err(|_| {
            InstanceError::PermissionError(s!("Unable to copy user exec file into box directory"))
        })?;

        fs::copy(
            self.runner_path.to_str().unwrap(),
            self.box_path.join("runner").to_str().unwrap(),
        )
        .map_err(|_| {
            InstanceError::PermissionError(s!("Unable to copy runner script into box directory"))
        })?;

        Ok(())
    }

    pub fn run(&self) -> Result<InstanceResult, InstanceError> {
        let args = self.get_arguments()?;
        let box_output = Command::new(get_env("ISOLATE_PATH")?)
            .args(args)
            .output()
            .map_err(|_| InstanceError::PermissionError(s!("Unable to run isolate.")))?;

        Ok(InstanceResult {
            status: RunVerdict::OK,
            time_usage: 1,
            memory_usage: 1,
        })
    }

    pub fn cleanup(&self) -> Result<(), InstanceError> {
        Command::new(get_env("ISOLATE_PATH")?)
            .args(&["--cleanup", "--cg", "-b"])
            .arg(self.box_id.to_string())
            .output()
            .map_err(|_| {
                InstanceError::PermissionError(s!("Unable to cleanup isolate --cleanup command."))
            })?;

        Command::new("rm")
            .arg(self.log_file.to_str().unwrap())
            .output()
            .map_err(|_| InstanceError::PermissionError(s!("Unable to remove log file.")))?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::instance::error::InstanceError;
    use crate::instance::model::{get_env, Instance};
    use dotenv::dotenv;
    use std::env;
    use std::fs;
    use std::path::PathBuf;
    use std::process::Command;

    #[test]
    fn declare_variable() -> Result<(), InstanceError> {
        let test_id = 1;
        let instance = Instance {
            box_id: test_id,
            bin_path: PathBuf::from("/path/to/bin"),
            time_limit: 1.0,
            memory_limit: 512000,
            input_path: PathBuf::from("/path/to/in"),
            output_path: PathBuf::from("/path/to/out"),
            runner_path: PathBuf::from("/path/to/runner"),
            ..Default::default()
        };
        Ok(())
    }

    #[test]
    fn initialize_instance() -> Result<(), InstanceError> {
        let test_id = 2;
        dotenv().ok();
        // get base directory
        let base_dir = PathBuf::from(env::current_dir().unwrap())
            .join("tests")
            .join("instance");

        let tmp_dir =
            PathBuf::from(get_env("TEMPORARY_PATH")?).join(format!("test_tmp_file_{}", test_id));

        if !tmp_dir.is_dir() {
            fs::create_dir(&tmp_dir).map_err(|_| {
                InstanceError::PermissionError(s!("Unable to create tmp directory"))
            })?;
        }

        Command::new(base_dir.join("compile_cpp").to_str().unwrap())
            .arg(tmp_dir.to_str().unwrap())
            .arg(base_dir.join("a_plus_b.cpp").to_str().unwrap())
            .output()
            .map_err(|_| InstanceError::PermissionError(s!("Unable to compile file")))?;

        let mut instance = Instance {
            box_id: test_id,
            bin_path: tmp_dir.join("bin"),
            time_limit: 1.0,
            memory_limit: 512000,
            input_path: base_dir.join("input.txt"),
            output_path: base_dir.join("output.txt"),
            runner_path: base_dir.join("run_cpp"),
            ..Default::default()
        };

        let result = || -> Result<(), InstanceError> {
            instance.cleanup()?;
            instance.init()?;
            instance.run()?;
            Ok(())
        }();

        // clean up
        fs::remove_dir_all(&tmp_dir)
            .map_err(|_| InstanceError::PermissionError(s!("Unable to remove tmp directory")))?;

        instance.cleanup()?;
        result
    }

    #[test]
    fn should_error_if_input_path_is_wrong() -> Result<(), InstanceError> {
        let test_id = 3;
        dotenv().ok();
        // get base directory
        let base_dir = PathBuf::from(env::current_dir().unwrap())
            .join("tests")
            .join("instance");

        let tmp_dir =
            PathBuf::from(get_env("TEMPORARY_PATH")?).join(format!("test_tmp_file_{}", test_id));

        if !tmp_dir.is_dir() {
            fs::create_dir(&tmp_dir).map_err(|_| {
                InstanceError::PermissionError(s!("Unable to create tmp directory"))
            })?;
        }

        // compile cpp first
        Command::new(base_dir.join("compile_cpp").to_str().unwrap())
            .arg(tmp_dir.to_str().unwrap())
            .arg(base_dir.join("a_plus_b.cpp").to_str().unwrap())
            .output()
            .map_err(|_| InstanceError::PermissionError(s!("Unable to compile files")))?;

        let mut instance = Instance {
            box_id: test_id,
            bin_path: tmp_dir.join("bin"),
            time_limit: 1.0,
            memory_limit: 512000,
            input_path: base_dir.join("input.txt_wrong_path"),
            output_path: base_dir.join("output.txt"),
            runner_path: base_dir.join("run_cpp"),
            ..Default::default()
        };

        instance.cleanup()?;

        let init_result = instance.init();

        fs::remove_dir_all(&tmp_dir)
            .map_err(|_| InstanceError::PermissionError(s!("Unable to remove tmp directory")))?;

        instance.cleanup()?;

        assert_eq!(
            init_result,
            Err(InstanceError::PermissionError(s!(
                "Unable to copy input file into box directory"
            )))
        );
        Ok(())
    }

    #[test]
    fn should_error_if_output_path_is_wrong() -> Result<(), InstanceError> {
        let test_id = 4;
        dotenv().ok();
        // get base directory
        let base_dir = PathBuf::from(env::current_dir().unwrap())
            .join("tests")
            .join("instance");

        let tmp_dir =
            PathBuf::from(get_env("TEMPORARY_PATH")?).join(format!("test_tmp_file_{}", test_id));

        if !tmp_dir.is_dir() {
            fs::create_dir(&tmp_dir).map_err(|_| {
                InstanceError::PermissionError(s!("Unable to create tmp directory"))
            })?;
        }

        // compile cpp first
        Command::new(base_dir.join("compile_cpp").to_str().unwrap())
            .arg(tmp_dir.to_str().unwrap())
            .arg(base_dir.join("a_plus_b.cpp").to_str().unwrap())
            .output()
            .map_err(|_| InstanceError::PermissionError(s!("Unable to compile files")))?;

        let mut instance = Instance {
            box_id: test_id,
            bin_path: tmp_dir.join("bin_wrong_path"),
            time_limit: 1.0,
            memory_limit: 512000,
            input_path: base_dir.join("input.txt"),
            output_path: base_dir.join("output.txt"),
            runner_path: base_dir.join("run_cpp"),
            ..Default::default()
        };

        instance.cleanup()?;

        let init_result = instance.init();

        fs::remove_dir_all(&tmp_dir)
            .map_err(|_| InstanceError::PermissionError(s!("Unable to remove tmp directory")))?;

        assert_eq!(
            init_result,
            Err(InstanceError::PermissionError(s!(
                "Unable to copy user exec file into box directory"
            )))
        );
        Ok(())
    }

    #[test]
    fn should_error_if_runner_path_is_wrong() -> Result<(), InstanceError> {
        let test_id = 5;
        dotenv().ok();
        // get base directory
        let base_dir = PathBuf::from(env::current_dir().unwrap())
            .join("tests")
            .join("instance");

        let tmp_dir =
            PathBuf::from(get_env("TEMPORARY_PATH")?).join(format!("test_tmp_file_{}", test_id));

        if !tmp_dir.is_dir() {
            fs::create_dir(&tmp_dir).map_err(|_| {
                InstanceError::PermissionError(s!("Unable to create tmp directory"))
            })?;
        }

        // compile cpp first
        Command::new(base_dir.join("compile_cpp").to_str().unwrap())
            .arg(tmp_dir.to_str().unwrap())
            .arg(base_dir.join("a_plus_b.cpp").to_str().unwrap())
            .output()
            .map_err(|_| InstanceError::PermissionError(s!("Unable to compile files")))?;

        let mut instance = Instance {
            box_id: test_id,
            bin_path: tmp_dir.join("bin"),
            time_limit: 1.0,
            memory_limit: 512000,
            input_path: base_dir.join("input.txt"),
            output_path: base_dir.join("output.txt"),
            runner_path: base_dir.join("run_cpp_wrong_path"),
            ..Default::default()
        };

        instance.cleanup()?;

        let init_result = instance.init();

        fs::remove_dir_all(&tmp_dir)
            .map_err(|_| InstanceError::PermissionError(s!("Unable to remove tmp directory")))?;

        assert_eq!(
            init_result,
            Err(InstanceError::PermissionError(s!(
                "Unable to copy runner script into box directory"
            )))
        );
        Ok(())
    }
}
