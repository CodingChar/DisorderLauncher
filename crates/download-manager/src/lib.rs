mod error;
mod manager;
pub mod archive;

pub use error::*;
pub use manager::*;

use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct DownloadTask {
    pub url: String,
    pub dest: PathBuf,
    pub size: Option<u64>,
    pub sha1: Option<String>,
}

#[derive(Debug, Clone)]
pub struct DownloadProgress {
    pub completed: usize,
    pub total: usize,
    pub bytes_downloaded: u64,
    pub bytes_total: u64,
    pub current_file: String,
}

#[derive(Debug, Clone)]
pub struct DownloadReport {
    pub total: usize,
    pub succeeded: usize,
    pub skipped: usize,
    pub failed: Vec<DownloadFailure>,
}

#[derive(Debug, Clone)]
pub struct DownloadFailure {
    pub url: String,
    pub dest: PathBuf,
    pub error: String,
}
