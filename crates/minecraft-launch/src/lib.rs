use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct LaunchPlan {
    pub java_path: PathBuf,
    pub main_class: String,
    pub classpath: Vec<PathBuf>,
    pub jvm_args: Vec<String>,
    pub game_args: Vec<String>,
    pub working_directory: PathBuf,
    pub environment: HashMap<String, String>,
}
