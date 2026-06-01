use std::{
    path::{Path, PathBuf},
    process::{Command, ExitStatus},
};

use sdk_mod_loader::{discover_mods, DiscoveredMod};

use crate::config::SdktConfig;

#[derive(Debug, Clone)]
pub(crate) struct LaunchPlan {
    pub(crate) game_path: PathBuf,
    pub(crate) mods_root: PathBuf,
    pub(crate) discovered_mods: Vec<DiscoveredMod>,
}

pub(crate) fn build_launch_plan(
    project_root: &Path,
    config: &SdktConfig,
) -> Result<LaunchPlan, String> {
    let game_path = config.game_path.as_ref().ok_or_else(|| {
        "missing game_path, set it with `sdkt config --game-path ...`".to_string()
    })?;
    let game_path = crate::config::resolve_path(project_root, game_path);
    if !game_path.exists() {
        return Err(format!("game_path does not exist: {}", game_path.display()));
    }

    let mods_root = config
        .mods_path
        .as_ref()
        .map(|mods_path| crate::config::resolve_path(project_root, mods_path))
        .unwrap_or_else(|| project_root.join("mods"));
    let discovered_mods = discover_mods(&mods_root);

    Ok(LaunchPlan {
        game_path,
        mods_root,
        discovered_mods,
    })
}

pub(crate) fn render_launch_plan(plan: &LaunchPlan) -> String {
    let mut output = String::new();
    output.push_str(&format!("game: {}\n", plan.game_path.display()));
    output.push_str(&format!("mods: {}\n", plan.mods_root.display()));
    output.push_str(&format!(
        "mods discovered: {}\n",
        plan.discovered_mods.len()
    ));
    for discovered in &plan.discovered_mods {
        output.push_str(&format!(
            "  - {} [{}]\n",
            discovered.manifest.name, discovered.manifest.id
        ));
    }
    output
}

pub(crate) fn spawn_game(plan: &LaunchPlan) -> Result<ExitStatus, String> {
    let mut command = Command::new(&plan.game_path);
    if let Some(parent) = plan.game_path.parent() {
        command.current_dir(parent);
    }
    command.env("OPPW4_SDK_MODS_ROOT", &plan.mods_root);
    command
        .status()
        .map_err(|error| format!("failed to launch game: {error}"))
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;

    #[test]
    fn builds_plan_from_mods_root() {
        let root = temp_root("launch-plan");
        fs::create_dir_all(root.join("mods").join("example")).expect("mods dir");
        fs::write(
            root.join("mods").join("example").join("mod.toml"),
            r#"
            [mod]
            id = "example"
            name = "Example"

            [uses]
            plugins = ["sdk_runtime"]

            [entry]
            file = "main.js"
        "#,
        )
        .expect("manifest");
        fs::write(root.join("mods").join("example").join("main.js"), "").expect("entry");
        fs::write(root.join("game.exe"), []).expect("game");

        let config = SdktConfig {
            game_path: Some(root.join("game.exe")),
            mods_path: Some(root.join("mods")),
            profile: Some("default".to_string()),
            default_bridge: Some("bridge-js".to_string()),
        };

        let plan = build_launch_plan(&root, &config).expect("plan");

        assert_eq!(plan.discovered_mods.len(), 1);
        assert_eq!(plan.discovered_mods[0].manifest.id, "example");
        let _ = fs::remove_dir_all(root);
    }

    #[cfg(unix)]
    #[test]
    fn spawns_game_with_mods_root_override() {
        use std::os::unix::fs::PermissionsExt;

        let root = temp_root("spawn-game");
        fs::create_dir_all(&root).expect("root");
        let script = root.join("game.sh");
        let env_file = root.join("mods-root.txt");
        fs::write(
            &script,
            format!(
                "#!/bin/sh\nprintf '%s' \"$OPPW4_SDK_MODS_ROOT\" > '{}'\nexit 7\n",
                env_file.display()
            ),
        )
        .expect("script");
        let mut perms = fs::metadata(&script).expect("metadata").permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&script, perms).expect("chmod");

        let plan = LaunchPlan {
            game_path: script.clone(),
            mods_root: root.join("mods"),
            discovered_mods: Vec::new(),
        };

        let status = spawn_game(&plan).expect("spawn");

        assert_eq!(status.code(), Some(7));
        assert_eq!(
            fs::read_to_string(env_file).expect("env"),
            root.join("mods").display().to_string()
        );
        let _ = fs::remove_dir_all(root);
    }

    fn temp_root(label: &str) -> std::path::PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("time")
            .as_nanos();
        std::env::temp_dir().join(format!("oppw4-sdkt-{label}-{nanos}"))
    }
}
