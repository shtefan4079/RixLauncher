use crate::launcher::constants;
use crate::launcher::types::{LoaderProfile, MinecraftInstance, VersionInfo};
use crate::launcher::utils::{
    get_library_path, get_native_library, offline_uuid, publish_replacement, replace_variables,
    safe_relative_component, safe_relative_path, should_allow_library, unique_temp_path,
};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

pub fn launch_instance(
    instance: &MinecraftInstance,
    active_player: &str,
    active_uuid: Option<&str>,
    access_token: Option<&str>,
    is_microsoft: bool,
    extra_jvm_args: &str,
    extra_game_args: &str,
    java_path: &str,
    memory_gb: f32,
    game_root: &str,
) -> Result<std::process::Child, String> {
    let loader = constants::canonical_loader(&instance.loader).ok_or_else(|| {
        format!(
            "Loader '{}' is not supported. Choose Vanilla, Fabric, or Quilt.",
            instance.loader
        )
    })?;
    let version = instance.version.trim();
    let Some(version_component) = safe_relative_component(version) else {
        return Err("Minecraft version has an invalid identifier".to_string());
    };
    if game_root.trim().is_empty() {
        return Err("Game directory cannot be empty".to_string());
    }

    // Determine directories
    let game_root_path = PathBuf::from(game_root.trim());
    let dot_rixlauncher = game_root_path.parent().unwrap_or(&game_root_path);
    let libraries_dir = dot_rixlauncher.join("libraries");
    let assets_dir = dot_rixlauncher.join("assets");
    let versions_dir = dot_rixlauncher.join("versions");

    // 1. Load vanilla version JSON
    let vanilla_json_path = versions_dir
        .join(&version_component)
        .join(format!("{version}.json"));
    if !vanilla_json_path.exists() {
        return Err(format!(
            "Missing version configuration. Recreate the instance."
        ));
    }

    let vanilla_json_content = fs::read_to_string(&vanilla_json_path)
        .map_err(|e| format!("Failed to read version config: {e}"))?;

    let vanilla_info: VersionInfo = serde_json::from_str(&vanilla_json_content)
        .map_err(|e| format!("Failed to parse version config: {e}"))?;
    if vanilla_info.id != version {
        return Err("Version configuration does not match the selected version".to_string());
    }

    // 2. Load loader profile if not Vanilla
    let mut loader_profile: Option<LoaderProfile> = None;
    if loader != constants::VANILLA_LOADER {
        let profile_path = instance.path.join("profile.json");
        if !profile_path.is_file() {
            return Err(format!(
                "Missing loader profile for {}. Recreate the instance.",
                loader
            ));
        }
        let content = fs::read_to_string(&profile_path)
            .map_err(|e| format!("Failed to read loader profile: {e}"))?;
        let profile: LoaderProfile = serde_json::from_str(&content)
            .map_err(|e| format!("Failed to parse loader profile: {e}"))?;
        if profile.inherits_from.as_deref() != Some(version)
            || !loader_profile_matches_loader(&profile, loader)
        {
            return Err("Loader profile does not match the selected Minecraft version".to_string());
        }
        loader_profile = Some(profile);
    }

    // 3. Extract natives (if legacy version needs it)
    let natives_dir = instance.path.join("natives");
    fs::create_dir_all(&natives_dir)
        .map_err(|error| format!("Could not create natives directory: {error}"))?;

    let mut libraries_to_use = Vec::new();
    // Vanilla libraries
    for lib in &vanilla_info.libraries {
        if should_allow_library(lib.rules.as_deref().unwrap_or(&[])) {
            libraries_to_use.push(lib.clone());
        }
    }
    // Loader libraries
    if let Some(profile) = &loader_profile {
        for lib in &profile.libraries {
            if should_allow_library(lib.rules.as_deref().unwrap_or(&[])) {
                libraries_to_use.push(lib.clone());
            }
        }
    }

    // Check each library. If it has a native classifier for Linux, extract it!
    for lib in &libraries_to_use {
        if let Some(native_art) = get_native_library(lib) {
            if let Some(path_str) = &native_art.path {
                let Some(path) = safe_relative_path(path_str) else {
                    continue;
                };
                let local_jar = libraries_dir.join(path);
                if local_jar.exists() {
                    if let Err(e) = extract_natives(&local_jar, &natives_dir) {
                        println!(
                            "Warning: failed to extract native library {:?}: {}",
                            local_jar, e
                        );
                    }
                }
            }
        }
    }

    // 4. Construct Classpath
    let mut classpath_items = Vec::new();
    for lib in &libraries_to_use {
        if let Some(path) = get_library_path(lib) {
            let full_path = libraries_dir.join(path);
            if full_path.exists() {
                classpath_items.push(full_path.display().to_string());
            } else {
                println!("Warning: library not found: {}", full_path.display());
            }
        }
    }

    // Add client jar to classpath
    let client_jar_path = versions_dir
        .join(&version_component)
        .join(format!("{version}.jar"));
    if !client_jar_path.exists() {
        return Err(format!(
            "Minecraft client JAR not found at {}",
            client_jar_path.display()
        ));
    }
    classpath_items.push(client_jar_path.display().to_string());
    let classpath_str = classpath_items.join(if cfg!(target_os = "windows") {
        ";"
    } else {
        ":"
    });

    // 5. Build variables Map
    let mut vars = HashMap::new();
    let offline_uuid = offline_uuid(active_player);

    let final_uuid = active_uuid.unwrap_or(&offline_uuid);
    let final_token = access_token.unwrap_or(constants::OFFLINE_ACCESS_TOKEN);
    let user_type = if is_microsoft { "msa" } else { "legacy" };

    vars.insert("auth_player_name", active_player.to_string());
    vars.insert("version_name", version.to_string());
    vars.insert("game_directory", instance.path.display().to_string());
    vars.insert("assets_root", assets_dir.display().to_string());
    vars.insert("assets_index_name", vanilla_info.asset_index.id.clone());
    vars.insert("auth_uuid", final_uuid.to_string());
    vars.insert("auth_access_token", final_token.to_string());
    vars.insert("user_type", user_type.to_string());
    vars.insert("version_type", vanilla_info.version_type.clone());
    vars.insert("natives_directory", natives_dir.display().to_string());
    vars.insert("launcher_name", constants::APP_NAME.to_string());
    vars.insert("launcher_version", constants::APP_VERSION.to_string());
    vars.insert("classpath", classpath_str.clone());
    vars.insert("user_properties", "{}".to_string());
    vars.insert("clientid", constants::EMPTY_AUTH_VALUE.to_string());
    vars.insert("auth_xuid", constants::EMPTY_AUTH_VALUE.to_string());

    // 6. Build Arguments
    let mut jvm_args = Vec::new();
    let mut game_args = Vec::new();

    // Memory configuration: set both -Xms and -Xmx to avoid heap resizing stutters
    let memory_gb = memory_gb.clamp(
        crate::launcher::constants::MIN_MEMORY_GB,
        crate::launcher::constants::MAX_MEMORY_GB,
    );
    let heap_max = format!("-Xmx{:.0}G", memory_gb);
    let heap_min = format!("-Xms{:.0}G", memory_gb);
    jvm_args.push(heap_max);
    jvm_args.push(heap_min);

    // Keep user input as arguments. A value containing `=` is not silently
    // interpreted as an environment variable.
    let extra_jvm_parsed = split_command_line(extra_jvm_args)?;

    // 1. Process Vanilla arguments
    if let Some(args_info) = vanilla_info.arguments.as_ref() {
        if let Some(jvm_list) = &args_info.jvm {
            for val in jvm_list {
                val.to_arguments(&mut jvm_args);
            }
        } else {
            jvm_args.push("-Djava.library.path=${natives_directory}".to_string());
            jvm_args.push("-cp".to_string());
            jvm_args.push("${classpath}".to_string());
        }

        if let Some(game_list) = &args_info.game {
            for val in game_list {
                val.to_arguments(&mut game_args);
            }
        }
    } else {
        // Legacy arguments (1.12.2 and below)
        jvm_args.push("-Djava.library.path=${natives_directory}".to_string());
        jvm_args.push("-cp".to_string());
        jvm_args.push("${classpath}".to_string());

        if let Some(mc_args) = &vanilla_info.minecraft_arguments {
            game_args.extend(split_command_line(mc_args)?);
        }
    }

    // 2. Append Loader arguments (if present)
    if let Some(profile) = &loader_profile {
        if let Some(args_info) = &profile.arguments {
            if let Some(jvm_list) = &args_info.jvm {
                for val in jvm_list {
                    val.to_arguments(&mut jvm_args);
                }
            }
            if let Some(game_list) = &args_info.game {
                for val in game_list {
                    val.to_arguments(&mut game_args);
                }
            }
        }
    }

    // Append extra user JVM arguments
    jvm_args.extend(extra_jvm_parsed);
    if !extra_game_args.trim().is_empty() {
        game_args.extend(split_command_line(extra_game_args)?);
    }

    // Replace placeholders in all arguments
    let raw_jvm_args: Vec<String> = jvm_args
        .iter()
        .map(|arg| replace_variables(arg, &vars))
        .collect();
    let raw_game_args: Vec<String> = game_args
        .iter()
        .map(|arg| replace_variables(arg, &vars))
        .collect();

    // Clean up unresolved placeholders (and their preceding flags)
    let mut final_jvm_args = Vec::new();
    for arg in raw_jvm_args {
        if !arg.contains("${") || !arg.contains('}') {
            final_jvm_args.push(arg);
        }
    }

    let mut final_game_args = Vec::new();
    let mut i = 0;
    while i < raw_game_args.len() {
        let arg = &raw_game_args[i];

        // Remove an option together with its unresolved value, but do not
        // discard a normal value that happens to precede a placeholder.
        if i + 1 < raw_game_args.len() {
            let next_arg = &raw_game_args[i + 1];
            if next_arg.contains("${") && next_arg.contains('}') && arg.starts_with('-') {
                i += 2;
                continue;
            }
        }

        // If the current argument itself is unresolved, skip it
        if arg.contains("${") && arg.contains('}') {
            i += 1;
            continue;
        }

        final_game_args.push(arg.clone());
        i += 1;
    }

    // 7. Choose entrypoint class
    let main_class = loader_profile
        .as_ref()
        .map(|p| p.main_class.clone())
        .unwrap_or_else(|| vanilla_info.main_class.clone());

    // 8. Launch!
    let mut command = Command::new(java_path);
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(constants::WINDOWS_CREATE_NO_WINDOW);
    }
    command.args(&final_jvm_args);
    command.arg(&main_class);
    command.args(&final_game_args);
    command.current_dir(&instance.path);

    // Keep game output in a rotating per-instance log instead of dropping it.
    let logs_dir = instance.path.join("logs");
    fs::create_dir_all(&logs_dir)
        .map_err(|error| format!("Could not create game logs directory: {error}"))?;
    let log_path = logs_dir.join("latest.log");
    rotate_log(&log_path).map_err(|e| format!("Could not prepare game log: {e}"))?;
    let stdout_log = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)
        .map_err(|e| format!("Could not open game log: {e}"))?;
    let stderr_log = stdout_log
        .try_clone()
        .map_err(|e| format!("Could not duplicate game log handle: {e}"))?;
    command.stdin(std::process::Stdio::null());
    command.stdout(std::process::Stdio::from(stdout_log));
    command.stderr(std::process::Stdio::from(stderr_log));

    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        // Run in a new process group so parent crashes or terminal signals (e.g. Ctrl+C) do not kill Minecraft
        command.process_group(0);
    }

    eprintln!(
        "[RixLauncher] Starting {} with {} JVM args and {} game args",
        instance.name,
        final_jvm_args.len(),
        final_game_args.len()
    );

    command
        .spawn()
        .map_err(|e| format!("Could not start Java: {e}"))
}

fn loader_profile_matches_loader(profile: &LoaderProfile, loader: &str) -> bool {
    let profile_id = profile.id.to_ascii_lowercase();
    let main_class = profile.main_class.to_ascii_lowercase();
    match loader {
        constants::FABRIC_LOADER => {
            profile_id.contains("fabric") || main_class.contains("fabricmc")
        }
        constants::QUILT_LOADER => profile_id.contains("quilt") || main_class.contains("quiltmc"),
        _ => false,
    }
}

fn split_command_line(input: &str) -> Result<Vec<String>, String> {
    let mut args = Vec::new();
    let mut current = String::new();
    let mut quote = None;
    let mut escaped = false;
    let mut had_content = false;

    for character in input.chars() {
        if escaped {
            current.push(character);
            escaped = false;
            had_content = true;
            continue;
        }
        match quote {
            Some('"') if character == '"' => quote = None,
            Some('\'') if character == '\'' => quote = None,
            Some(_) if character == '\\' => escaped = true,
            Some(_) => {
                current.push(character);
                had_content = true;
            }
            None if character == '"' || character == '\'' => quote = Some(character),
            None if character == '\\' => escaped = true,
            None if character.is_whitespace() => {
                if had_content {
                    args.push(std::mem::take(&mut current));
                    had_content = false;
                }
            }
            None => {
                current.push(character);
                had_content = true;
            }
        }
    }
    if escaped {
        return Err("Extra arguments end with an unfinished escape".to_string());
    }
    if quote.is_some() {
        return Err("Extra arguments contain an unfinished quote".to_string());
    }
    if had_content {
        args.push(current);
    }
    Ok(args)
}

fn rotate_log(path: &Path) -> std::io::Result<()> {
    if path.metadata().map(|metadata| metadata.len()).unwrap_or(0)
        < crate::launcher::constants::MAX_GAME_LOG_BYTES
    {
        return Ok(());
    }
    let first = path.with_extension("log.1");
    let second = path.with_extension("log.2");
    let _ = fs::remove_file(&second);
    if first.exists() {
        let _ = fs::rename(&first, &second);
    }
    fs::rename(path, first)
}

/// Stops the Java process and the process group/job it owns. Minecraft often
/// launches a separate native helper, so terminating only the direct child
/// leaves the game running in the background.
pub fn stop_process_tree(child: &mut std::process::Child) {
    let pid = child.id();
    #[cfg(unix)]
    {
        // The launcher starts Java in a new process group. A negative PID
        // targets that group; the direct kill below is the safe fallback if
        // the group has already disappeared.
        if let Ok(group_id) = i32::try_from(pid) {
            unsafe {
                let _ = libc::kill(-group_id, libc::SIGTERM);
            }
        }
    }
    #[cfg(windows)]
    {
        let _ = std::process::Command::new("taskkill")
            .args(["/PID", &pid.to_string(), "/T", "/F"])
            .status();
    }
    let _ = child.kill();
    let _ = child.wait();
}

pub fn extract_natives(jar_path: &Path, dest_dir: &Path) -> Result<(), String> {
    let file = fs::File::open(jar_path).map_err(|e| e.to_string())?;
    let mut archive = zip::ZipArchive::new(file).map_err(|e| e.to_string())?;
    fs::create_dir_all(dest_dir).map_err(|e| e.to_string())?;

    for i in 0..archive.len() {
        let mut file = archive.by_index(i).map_err(|e| e.to_string())?;
        let name = file.name();
        let is_native = if cfg!(target_os = "windows") {
            name.ends_with(".dll")
        } else if cfg!(target_os = "macos") {
            name.ends_with(".dylib")
        } else {
            name.ends_with(".so") || name.contains(".so.")
        };
        if is_native {
            let file_name = Path::new(file.name())
                .file_name()
                .unwrap_or(std::ffi::OsStr::new(file.name()));
            let outpath = dest_dir.join(file_name);
            let temporary = unique_temp_path(&outpath, "native");
            let write_result = (|| -> Result<(), String> {
                let mut outfile = fs::File::create(&temporary).map_err(|e| e.to_string())?;
                std::io::copy(&mut file, &mut outfile).map_err(|e| e.to_string())?;
                outfile.sync_all().map_err(|e| e.to_string())?;
                publish_replacement(&temporary, &outpath).map_err(|e| e.to_string())
            })();
            if let Err(error) = write_result {
                let _ = fs::remove_file(&temporary);
                return Err(error);
            }
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let mut perms = fs::metadata(&outpath)
                    .map_err(|e| e.to_string())?
                    .permissions();
                perms.set_mode(0o755);
                fs::set_permissions(&outpath, perms).map_err(|e| e.to_string())?;
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::split_command_line;

    #[test]
    fn extra_arguments_preserve_quoted_values() {
        let args = split_command_line(r#"-Dfoo="bar baz" --flag 'two words'"#).unwrap();
        assert_eq!(args, ["-Dfoo=bar baz", "--flag", "two words"]);
    }

    #[test]
    fn extra_arguments_report_unfinished_quotes() {
        assert!(split_command_line("--flag \"unfinished").is_err());
        assert!(split_command_line("--flag\\").is_err());
    }
}
