use crate::rules::{current_os_name, rules_allow};
use crate::{Artifact, InstallPlan, NativeExtraction};
use minecraft_meta::VersionDescriptor;
use std::path::Path;

fn native_arch_bits() -> &'static str {
    if cfg!(target_pointer_width = "64") {
        "64"
    } else {
        "32"
    }
}

pub fn build_install_plan(
    descriptor: &VersionDescriptor,
    game_dir: &Path,
) -> InstallPlan {
    let versions_dir = game_dir.join("versions").join(&descriptor.id);
    let libraries_dir = game_dir.join("libraries");
    let natives_dir = versions_dir.join("natives");

    let client = Artifact {
        url: descriptor.downloads.client.url.clone(),
        path: versions_dir.join(format!("{}.jar", descriptor.id)),
        size: Some(descriptor.downloads.client.size),
        sha1: Some(descriptor.downloads.client.sha1.clone()),
    };

    let asset_index = Artifact {
        url: descriptor.asset_index.url.clone(),
        path: game_dir
            .join("assets")
            .join("indexes")
            .join(format!("{}.json", descriptor.asset_index.id)),
        size: Some(descriptor.asset_index.size),
        sha1: Some(descriptor.asset_index.sha1.clone()),
    };

    let mut libraries = Vec::new();
    let mut natives = Vec::new();
    let mut native_extractions = Vec::new();

    for lib in &descriptor.libraries {
        if let Some(rules) = &lib.rules {
            if !rules_allow(rules) {
                continue;
            }
        }

        if let Some(downloads) = &lib.downloads {
            if let Some(artifact) = &downloads.artifact {
                libraries.push(Artifact {
                    url: artifact.url.clone(),
                    path: libraries_dir.join(&artifact.path),
                    size: Some(artifact.size),
                    sha1: Some(artifact.sha1.clone()),
                });
            }

            if let Some(native_key) = lib
                .natives
                .as_ref()
                .and_then(|n| n.get(current_os_name()))
                .map(|key| key.replace("${arch}", native_arch_bits()))
            {
                if let Some(native_artifact) = downloads
                    .classifiers
                    .as_ref()
                    .and_then(|c| c.get(&native_key))
                {
                    let jar = libraries_dir.join(&native_artifact.path);
                    natives.push(Artifact {
                        url: native_artifact.url.clone(),
                        path: jar.clone(),
                        size: Some(native_artifact.size),
                        sha1: Some(native_artifact.sha1.clone()),
                    });
                    native_extractions.push(NativeExtraction {
                        jar,
                        exclude: lib
                            .extract
                            .as_ref()
                            .map(|e| e.exclude.clone())
                            .unwrap_or_default(),
                    });
                }
            }
        }
    }

    let java_major_version = descriptor
        .java_version
        .as_ref()
        .map(|jv| jv.major_version);

    InstallPlan {
        version: descriptor.id.clone(),
        client,
        libraries,
        asset_index,
        assets: Vec::new(),
        natives,
        native_extractions,
        natives_dir,
        java_major_version,
    }
}

pub fn build_asset_artifacts(
    asset_index: &minecraft_meta::AssetIndex,
    game_dir: &Path,
) -> Vec<Artifact> {
    let objects_dir = game_dir.join("assets").join("objects");

    asset_index
        .objects
        .values()
        .map(|obj| {
            let prefix = &obj.hash[..2];
            Artifact {
                url: format!(
                    "https://resources.download.minecraft.net/{}/{}",
                    prefix, obj.hash
                ),
                path: objects_dir.join(prefix).join(&obj.hash),
                size: Some(obj.size),
                sha1: Some(obj.hash.clone()),
            }
        })
        .collect()
}
