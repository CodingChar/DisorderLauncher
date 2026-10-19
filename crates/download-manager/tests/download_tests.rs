use download_manager::*;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

fn make_task(url: &str, dest: std::path::PathBuf, sha1: Option<&str>) -> DownloadTask {
    DownloadTask {
        url: url.to_string(),
        dest,
        size: None,
        sha1: sha1.map(String::from),
    }
}

#[tokio::test]
async fn rejects_http_url() {
    let dir = tempfile::tempdir().unwrap();
    let manager = DownloadManager::new(reqwest::Client::new());

    let task = make_task(
        "http://example.com/file.jar",
        dir.path().join("file.jar"),
        None,
    );

    let report = manager
        .download_all(vec![task], 1, |_| {})
        .await
        .unwrap();

    assert_eq!(report.failed.len(), 1);
    assert!(report.failed[0].error.contains("https"));
}

#[tokio::test]
async fn skips_already_downloaded_with_matching_hash() {
    let dir = tempfile::tempdir().unwrap();
    let dest = dir.path().join("existing.jar");
    let content = b"existing file content";

    tokio::fs::write(&dest, content).await.unwrap();

    let sha1 = {
        use sha1::{Digest, Sha1};
        let mut h = Sha1::new();
        h.update(content);
        format!("{:x}", h.finalize())
    };

    let manager = DownloadManager::new(reqwest::Client::new());
    let task = make_task(
        "https://example.com/wont-be-fetched.jar",
        dest.clone(),
        Some(&sha1),
    );

    let report = manager
        .download_all(vec![task], 1, |_| {})
        .await
        .unwrap();

    assert_eq!(report.skipped, 1);
    assert_eq!(report.succeeded, 0);
    assert!(report.failed.is_empty());
}

#[tokio::test]
async fn progress_callback_fires() {
    let dir = tempfile::tempdir().unwrap();
    let dest = dir.path().join("existing.jar");
    let content = b"data";
    tokio::fs::write(&dest, content).await.unwrap();

    let sha1 = {
        use sha1::{Digest, Sha1};
        let mut h = Sha1::new();
        h.update(content);
        format!("{:x}", h.finalize())
    };

    let count = Arc::new(AtomicUsize::new(0));
    let count_clone = count.clone();

    let manager = DownloadManager::new(reqwest::Client::new());
    let task = make_task(
        "https://example.com/skip.jar",
        dest,
        Some(&sha1),
    );

    manager
        .download_all(vec![task], 1, move |_p| {
            count_clone.fetch_add(1, Ordering::Relaxed);
        })
        .await
        .unwrap();

    assert!(count.load(Ordering::Relaxed) >= 1);
}

#[tokio::test]
async fn empty_task_list() {
    let manager = DownloadManager::new(reqwest::Client::new());
    let report = manager.download_all(vec![], 4, |_| {}).await.unwrap();
    assert_eq!(report.total, 0);
    assert_eq!(report.succeeded, 0);
    assert!(report.failed.is_empty());
}

#[tokio::test]
async fn zero_concurrency_does_not_hang() {
    let dir = tempfile::tempdir().unwrap();
    let manager = DownloadManager::new(reqwest::Client::new());
    let task = make_task("http://example.com/file", dir.path().join("file"), None);
    let report = tokio::time::timeout(
        std::time::Duration::from_secs(2),
        manager.download_all(vec![task], 0, |_| {}),
    ).await.expect("zero concurrency blocked the downloader").unwrap();
    assert_eq!(report.failed.len(), 1);
}

#[tokio::test]
async fn rejected_hosts_still_report_completion() {
    let dir = tempfile::tempdir().unwrap();
    let manager = DownloadManager::new(reqwest::Client::new())
        .with_allowed_host_suffixes(&["mojang.com"]);
    let progress = Arc::new(std::sync::Mutex::new(Vec::new()));
    let captured = progress.clone();
    let report = manager.download_all(
        vec![make_task("https://example.com/file", dir.path().join("file"), None)],
        1,
        move |p| captured.lock().unwrap().push(p),
    ).await.unwrap();
    assert_eq!(report.failed.len(), 1);
    let events = progress.lock().unwrap();
    assert_eq!(events.last().unwrap().completed, 1);
    assert_eq!(events.last().unwrap().total, 1);
}
