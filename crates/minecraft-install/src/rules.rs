use minecraft_meta::Rule;

pub fn current_os_name() -> &'static str {
    if cfg!(target_os = "windows") {
        "windows"
    } else if cfg!(target_os = "macos") {
        "osx"
    } else {
        "linux"
    }
}

pub fn current_arch() -> &'static str {
    if cfg!(target_arch = "x86_64") {
        "x86_64"
    } else if cfg!(target_arch = "x86") {
        "x86"
    } else if cfg!(target_arch = "aarch64") {
        "aarch64"
    } else {
        "unknown"
    }
}

pub fn rules_allow(rules: &[Rule]) -> bool {
    let mut allowed = false;

    for rule in rules {
        let mut matches = true;

        if let Some(os) = &rule.os {
            if let Some(name) = &os.name {
                if name != current_os_name() {
                    matches = false;
                }
            }
            if let Some(arch) = &os.arch {
                if arch != current_arch() {
                    matches = false;
                }
            }
        }

        if matches {
            allowed = rule.action == "allow";
        }
    }

    allowed
}
