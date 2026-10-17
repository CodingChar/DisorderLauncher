use mods::{pick_best_version, plan_install, ModError, ModVersion, VersionSource};
use std::collections::{HashMap, HashSet};

/// Fuente en memoria: proyecto -> versiones compatibles.
#[derive(Default)]
struct FakeSource {
    versions: HashMap<String, Vec<ModVersion>>,
    by_id: HashMap<String, ModVersion>,
    titles: HashMap<String, String>,
}

#[async_trait::async_trait]
impl VersionSource for FakeSource {
    async fn compatible_versions(&self, project_id: &str) -> Result<Vec<ModVersion>, ModError> {
        Ok(self.versions.get(project_id).cloned().unwrap_or_default())
    }

    async fn version_by_id(&self, version_id: &str) -> Result<ModVersion, ModError> {
        self.by_id.get(version_id).cloned().ok_or(ModError::NotFound)
    }

    async fn project_title(&self, project_id: &str) -> Result<String, ModError> {
        Ok(self.titles.get(project_id).cloned().unwrap_or_else(|| project_id.to_string()))
    }
}

fn version(project: &str, id: &str, kind: &str, date: &str, required: &[&str]) -> ModVersion {
    let deps: Vec<_> = required
        .iter()
        .map(|p| serde_json::json!({ "project_id": p, "version_id": null, "dependency_type": "required" }))
        .collect();
    serde_json::from_value(serde_json::json!({
        "id": id, "project_id": project, "version_number": id, "name": id,
        "date_published": date, "version_type": kind,
        "files": [{ "url": "https://cdn.modrinth.com/x.jar", "filename": format!("{id}.jar"), "primary": true, "size": 1, "hashes": { "sha1": "aa" } }],
        "dependencies": deps
    }))
    .unwrap()
}

fn source(entries: &[(&str, Vec<ModVersion>)]) -> FakeSource {
    let mut s = FakeSource::default();
    for (project, versions) in entries {
        s.titles.insert(project.to_string(), project.to_uppercase());
        for v in versions {
            s.by_id.insert(v.id.clone(), v.clone());
        }
        s.versions.insert(project.to_string(), versions.clone());
    }
    s
}

fn ids(plan: &[mods::PlannedMod]) -> Vec<&str> {
    plan.iter().map(|p| p.project_id.as_str()).collect()
}

#[test]
fn prefers_latest_release_over_newer_beta() {
    let versions = vec![
        version("p", "old", "release", "2024-01-01T00:00:00Z", &[]),
        version("p", "new", "release", "2024-06-01T00:00:00Z", &[]),
        version("p", "beta", "beta", "2024-09-01T00:00:00Z", &[]),
    ];
    assert_eq!(pick_best_version(&versions).unwrap().id, "new");

    let only_beta = vec![version("p", "b1", "beta", "2024-01-01T00:00:00Z", &[]), version("p", "a1", "alpha", "2025-01-01T00:00:00Z", &[])];
    assert_eq!(pick_best_version(&only_beta).unwrap().id, "b1");
    assert!(pick_best_version(&[]).is_none());
}

#[tokio::test]
async fn installs_dependencies_first_and_marks_them() {
    let src = source(&[
        ("sodium", vec![version("sodium", "s1", "release", "2024-01-01T00:00:00Z", &["fapi", "lib"])]),
        ("fapi", vec![version("fapi", "f1", "release", "2024-01-01T00:00:00Z", &["lib"])]),
        ("lib", vec![version("lib", "l1", "release", "2024-01-01T00:00:00Z", &[])]),
    ]);
    let plan = plan_install(&src, "sodium", &HashSet::new()).await.unwrap();

    assert_eq!(ids(&plan), ["lib", "fapi", "sodium"]);
    assert_eq!(plan[2].dependency_of, None);
    assert_eq!(plan[1].dependency_of.as_deref(), Some("sodium"));
}

#[tokio::test]
async fn skips_already_installed_dependencies() {
    let src = source(&[
        ("mod", vec![version("mod", "m1", "release", "2024-01-01T00:00:00Z", &["fapi"])]),
        ("fapi", vec![version("fapi", "f1", "release", "2024-01-01T00:00:00Z", &[])]),
    ]);
    let installed = HashSet::from(["fapi".to_string()]);
    let plan = plan_install(&src, "mod", &installed).await.unwrap();
    assert_eq!(ids(&plan), ["mod"]);
}

#[tokio::test]
async fn handles_dependency_cycles() {
    let src = source(&[
        ("a", vec![version("a", "a1", "release", "2024-01-01T00:00:00Z", &["b"])]),
        ("b", vec![version("b", "b1", "release", "2024-01-01T00:00:00Z", &["a"])]),
    ]);
    let plan = plan_install(&src, "a", &HashSet::new()).await.unwrap();
    assert_eq!(ids(&plan), ["b", "a"]);
}

#[tokio::test]
async fn incompatible_root_and_missing_dependency_have_distinct_errors() {
    let src = source(&[
        ("root", vec![version("root", "r1", "release", "2024-01-01T00:00:00Z", &["ghost"])]),
        ("ghost", vec![]),
        ("nothing", vec![]),
    ]);
    match plan_install(&src, "nothing", &HashSet::new()).await {
        Err(ModError::NotCompatible(title)) => assert_eq!(title, "NOTHING"),
        other => panic!("{other:?}"),
    }
    match plan_install(&src, "root", &HashSet::new()).await {
        Err(ModError::MissingDependency(title)) => assert_eq!(title, "GHOST"),
        other => panic!("{other:?}"),
    }
}

#[tokio::test]
async fn pinned_version_dependencies_are_resolved_by_version_id() {
    let mut pinned = version("lib", "l-pinned", "release", "2024-01-01T00:00:00Z", &[]);
    pinned.project_id = "lib".into();
    let mut root = version("root", "r1", "release", "2024-01-01T00:00:00Z", &[]);
    root.dependencies = serde_json::from_value(serde_json::json!([
        { "project_id": null, "version_id": "l-pinned", "dependency_type": "required" },
        { "project_id": "opt", "version_id": null, "dependency_type": "optional" },
        { "project_id": "inc", "version_id": null, "dependency_type": "incompatible" }
    ]))
    .unwrap();

    let mut src = source(&[("root", vec![root])]);
    src.by_id.insert("l-pinned".into(), pinned);

    let plan = plan_install(&src, "root", &HashSet::new()).await.unwrap();
    assert_eq!(ids(&plan), ["lib", "root"], "las dependencias opcionales e incompatibles no se instalan");
    assert_eq!(plan[0].version.id, "l-pinned");
}
