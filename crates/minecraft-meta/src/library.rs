use crate::Library;

/// Archivo de una libreria ya resuelto: de `downloads.artifact` (Mojang) o de las
/// coordenadas Maven + URL base (loaders).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedLibrary {
    /// Ruta relativa dentro de `libraries/`.
    pub path: String,
    pub url: String,
    pub sha1: Option<String>,
    pub size: Option<u64>,
}

/// `group:artifact:version[:classifier][@ext]` -> `group/path/artifact/version/artifact-version[-classifier].ext`.
/// Rechaza coordenadas con segmentos vacios o con separadores de ruta.
pub fn maven_path(name: &str) -> Option<String> {
    let (coords, ext) = name.split_once('@').unwrap_or((name, "jar"));
    let parts: Vec<&str> = coords.split(':').collect();
    if !(3..=4).contains(&parts.len()) {
        return None;
    }
    let safe = |s: &str| !s.is_empty() && !s.contains(['/', '\\']) && s != ".." && s != ".";
    if !parts.iter().all(|p| safe(p)) || !safe(ext) {
        return None;
    }
    let (group, artifact, version) = (parts[0], parts[1], parts[2]);
    let file = match parts.get(3) {
        Some(classifier) => format!("{artifact}-{version}-{classifier}.{ext}"),
        None => format!("{artifact}-{version}.{ext}"),
    };
    Some(format!("{}/{artifact}/{version}/{file}", group.replace('.', "/")))
}

pub fn resolve_library(lib: &Library) -> Option<ResolvedLibrary> {
    if let Some(artifact) = lib.downloads.as_ref().and_then(|d| d.artifact.as_ref()) {
        return Some(ResolvedLibrary {
            path: artifact.path.clone(),
            url: artifact.url.clone(),
            sha1: Some(artifact.sha1.clone()),
            size: Some(artifact.size),
        });
    }
    let base = lib.url.as_deref()?;
    let path = maven_path(&lib.name)?;
    Some(ResolvedLibrary {
        url: format!("{}/{path}", base.trim_end_matches('/')),
        path,
        sha1: lib.sha1.clone(),
        size: lib.size,
    })
}

/// Clave de deduplicacion: `group:artifact[:classifier]` (sin version), para que la version
/// del loader gane sobre la de vanilla cuando ambas aportan la misma libreria.
pub fn library_key(name: &str) -> String {
    let coords = name.split_once('@').map(|(c, _)| c).unwrap_or(name);
    let mut parts = coords.split(':');
    let group = parts.next().unwrap_or_default();
    let artifact = parts.next().unwrap_or_default();
    let _version = parts.next();
    match parts.next() {
        Some(classifier) => format!("{group}:{artifact}:{classifier}"),
        None => format!("{group}:{artifact}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lib(name: &str, url: Option<&str>) -> Library {
        serde_json::from_value(serde_json::json!({ "name": name, "url": url })).unwrap()
    }

    #[test]
    fn maven_paths() {
        assert_eq!(
            maven_path("net.fabricmc:fabric-loader:0.16.9").unwrap(),
            "net/fabricmc/fabric-loader/0.16.9/fabric-loader-0.16.9.jar"
        );
        assert_eq!(
            maven_path("org.lwjgl:lwjgl:3.3.3:natives-windows").unwrap(),
            "org/lwjgl/lwjgl/3.3.3/lwjgl-3.3.3-natives-windows.jar"
        );
        assert_eq!(maven_path("a.b:c:1@zip").unwrap(), "a/b/c/1/c-1.zip");
    }

    #[test]
    fn maven_paths_reject_unsafe_coordinates() {
        for bad in ["a:b", "a:b:../../x", "a::1", "a:b:c:d:e", "../x:y:1", "a:b:1/2", "a:b:1@../x"] {
            assert!(maven_path(bad).is_none(), "{bad}");
        }
    }

    #[test]
    fn resolves_maven_style_libraries() {
        let r = resolve_library(&lib("net.fabricmc:intermediary:1.21", Some("https://maven.fabricmc.net/"))).unwrap();
        assert_eq!(r.url, "https://maven.fabricmc.net/net/fabricmc/intermediary/1.21/intermediary-1.21.jar");
        assert_eq!(r.sha1, None);
    }

    #[test]
    fn library_without_download_info_is_unresolvable() {
        assert!(resolve_library(&lib("a:b:1", None)).is_none());
    }

    #[test]
    fn keys_ignore_version_but_keep_classifier() {
        assert_eq!(library_key("org.ow2.asm:asm:9.6"), "org.ow2.asm:asm");
        assert_eq!(library_key("org.lwjgl:lwjgl:3.3.3:natives-windows"), "org.lwjgl:lwjgl:natives-windows");
    }
}
