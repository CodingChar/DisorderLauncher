use diagnostics::{repair_installation, verify_installation, IssueKind};
use minecraft_install::{build_install_plan, InstallExecutor};
use minecraft_meta::VersionDescriptor;
use sha1::{Digest, Sha1};
use std::path::Path;

fn sha1_of(data: &[u8]) -> String {
    format!("{:x}", Sha1::digest(data))
}

/// Instalacion minima y consistente: cliente + 1 libreria + indice con 2 assets.
struct Fixture {
    dir: tempfile::TempDir,
    descriptor: VersionDescriptor,
    asset_hashes: Vec<String>,
}

fn fixture() -> Fixture {
    let dir = tempfile::tempdir().unwrap();
    let client = b"client-jar";
    let lib = b"library-jar";
    let asset_a = b"asset-a";
    let asset_b = b"asset-b";
    let (ha, hb) = (sha1_of(asset_a), sha1_of(asset_b));
    let index = format!(
        r#"{{"objects":{{"a.txt":{{"hash":"{ha}","size":{}}},"b.txt":{{"hash":"{hb}","size":{}}}}}}}"#,
        asset_a.len(),
        asset_b.len()
    );

    let descriptor: VersionDescriptor = serde_json::from_str(&format!(
        r#"{{
  "id": "1.99", "type": "release", "mainClass": "M",
  "downloads": {{ "client": {{ "sha1": "{}", "size": {}, "url": "https://x/c.jar" }} }},
  "assetIndex": {{ "id": "idx", "sha1": "{}", "size": {}, "totalSize": 1, "url": "https://x/i.json" }},
  "assets": "idx",
  "minecraftArguments": "",
  "libraries": [ {{ "name": "a:b:1", "downloads": {{ "artifact": {{
      "path": "a/b/1/b-1.jar", "sha1": "{}", "size": {}, "url": "https://x/l.jar" }} }} }} ]
}}"#,
        sha1_of(client),
        client.len(),
        sha1_of(index.as_bytes()),
        index.len(),
        sha1_of(lib),
        lib.len()
    ))
    .unwrap();

    let root = dir.path();
    write(&root.join("versions/1.99/1.99.jar"), client);
    write(&root.join("libraries/a/b/1/b-1.jar"), lib);
    write(&root.join("assets/indexes/idx.json"), index.as_bytes());
    for (hash, data) in [(&ha, &asset_a[..]), (&hb, &asset_b[..])] {
        write(&root.join("assets/objects").join(&hash[..2]).join(hash), data);
    }

    Fixture { dir, descriptor, asset_hashes: vec![ha, hb] }
}

fn write(path: &Path, data: &[u8]) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, data).unwrap();
}

#[tokio::test]
async fn healthy_installation_reports_no_issues() {
    let f = fixture();
    let plan = build_install_plan(&f.descriptor, f.dir.path());
    let report = verify_installation(&plan, f.dir.path()).await;

    assert!(report.is_healthy(), "{report:?}");
    assert_eq!(report.ok, 5);
}

#[tokio::test]
async fn detects_missing_and_corrupted_files() {
    let f = fixture();
    let root = f.dir.path();
    std::fs::remove_file(root.join("libraries/a/b/1/b-1.jar")).unwrap();
    let hash = &f.asset_hashes[0];
    std::fs::write(root.join("assets/objects").join(&hash[..2]).join(hash), b"tampered").unwrap();

    let plan = build_install_plan(&f.descriptor, root);
    let report = verify_installation(&plan, root).await;

    assert_eq!(report.missing(), 1);
    assert_eq!(report.corrupted(), 1);
    assert!(report.issues.iter().any(|i| i.kind == IssueKind::Missing && i.artifact.path.ends_with("b-1.jar")));
}

#[tokio::test]
async fn missing_asset_index_marks_assets_unchecked() {
    let f = fixture();
    let root = f.dir.path();
    std::fs::remove_file(root.join("assets/indexes/idx.json")).unwrap();

    let plan = build_install_plan(&f.descriptor, root);
    let report = verify_installation(&plan, root).await;

    assert!(report.assets_unchecked);
    assert!(!report.is_healthy());
    assert_eq!(report.missing(), 1);
}

#[tokio::test]
async fn repair_of_healthy_installation_is_a_noop_without_network() {
    let f = fixture();
    let mut plan = build_install_plan(&f.descriptor, f.dir.path());
    let executor = InstallExecutor::new(reqwest::Client::new(), 2);

    let report = repair_installation(&executor, &mut plan, f.dir.path(), |_| {}).await.unwrap();
    assert_eq!(report.found, 0);
    assert!(report.is_fully_repaired());
}

/// Medicion de referencia (no es un test de CI): verificar ~2000 assets de 64 KB (~125 MB).
/// `cargo test -p diagnostics --test verify --release -- --ignored --nocapture perf`
#[tokio::test]
#[ignore = "medicion de rendimiento"]
async fn perf_verify_two_thousand_assets() {
    let dir = tempfile::tempdir().unwrap();
    let mut objects = Vec::new();
    for i in 0..2000u32 {
        let data = vec![(i % 251) as u8; 64 * 1024 + (i as usize % 7)];
        let hash = sha1_of(&data);
        write(&dir.path().join("assets/objects").join(&hash[..2]).join(&hash), &data);
        objects.push(format!("\"a{i}\":{{\"hash\":\"{hash}\",\"size\":{}}}", data.len()));
    }
    let index = format!("{{\"objects\":{{{}}}}}", objects.join(","));
    write(&dir.path().join("assets/indexes/idx.json"), index.as_bytes());

    let descriptor: minecraft_meta::VersionDescriptor = serde_json::from_str(&format!(
        r#"{{"id":"1.99","type":"release","mainClass":"M","downloads":{{"client":{{"sha1":"aa","size":1,"url":"https://x/c.jar"}}}},
        "assetIndex":{{"id":"idx","sha1":"{}","size":{},"totalSize":1,"url":"https://x/i.json"}},"assets":"idx","minecraftArguments":"","libraries":[]}}"#,
        sha1_of(index.as_bytes()),
        index.len()
    ))
    .unwrap();
    let plan = build_install_plan(&descriptor, dir.path());

    let started = std::time::Instant::now();
    let report = verify_installation(&plan, dir.path()).await;
    println!("verificados {} archivos en {:?} (problemas: {})", report.ok + report.issues.len(), started.elapsed(), report.issues.len());
    assert!(report.ok >= 2000);
}

#[test]
#[ignore = "medicion de rendimiento"]
fn perf_sha1_throughput_in_memory() {
    let data = vec![7u8; 64 * 1024];
    let started = std::time::Instant::now();
    for _ in 0..2000 {
        std::hint::black_box(sha1_of(&data));
    }
    let secs = started.elapsed().as_secs_f64();
    println!("sha1 en memoria: {:.0} MB/s", 125.0 / secs);
}
