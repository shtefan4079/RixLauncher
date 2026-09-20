use crate::launcher::constants;
use crate::launcher::java::{install_java_runtime, java_major, required_java_major};
use crate::launcher::mod_write_profile;
use crate::launcher::types::{
    AssetIndex, CreateRequest, LoaderProfile, VersionInfo, VersionManifest,
};
use crate::launcher::utils::{
    atomic_write, get_library_path, get_library_url, get_native_library, publish_replacement,
    safe_file_name, safe_relative_component, safe_relative_path, sanitize_name,
    should_allow_library, unique_temp_path, validate_remote_url,
};
use serde::de::DeserializeOwned;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

/// Preserves existing per-instance overrides (instance_java / instance_memory_gb)
/// when rewriting launch.properties, so re-preparing an instance never wipes
/// the user's per-instance settings.
fn read_existing_overrides(profile_dir: &Path) -> (Option<String>, Option<f32>) {
    let Ok(props) = crate::launcher::read_properties_file(&profile_dir.join("launch.properties"))
    else {
        return (None, None);
    };
    let java = props.get("instance_java").cloned().filter(|p| {
        !p.trim().is_empty()
            && !p
                .trim()
                .eq_ignore_ascii_case(constants::AUTO_JAVA_CONFIG_VALUE)
    });
    let memory = props
        .get("instance_memory_gb")
        .and_then(|v| v.parse::<f32>().ok())
        .filter(|v| *v > 0.0);
    (java, memory)
}

pub async fn create_instance_task(request: CreateRequest) -> Result<String, String> {
    if !is_valid_instance_name(&request.name) {
        return Err(
            "Instance name must contain 1–64 visible characters and no path separators".to_string(),
        );
    }
    let loader = constants::canonical_loader(&request.loader).ok_or_else(|| {
        "This loader is not supported yet. Choose Vanilla, Fabric, or Quilt.".to_string()
    })?;
    let version = validated_version_id(&request.version)?;

    if request.root.trim().is_empty() {
        return Err("Game directory cannot be empty".to_string());
    }
    let game_root = PathBuf::from(request.root.trim());
    let dot_rixlauncher = game_root.parent().unwrap_or(&game_root);

    let profile_dir = game_root.join(sanitize_name(&request.name));
    if profile_dir.exists() {
        return Err("Instance already exists".to_string());
    }

    // Create directories for shared data
    let libraries_dir = dot_rixlauncher.join("libraries");
    let assets_dir = dot_rixlauncher.join("assets");
    let versions_dir = dot_rixlauncher.join("versions");
    fs::create_dir_all(&libraries_dir).map_err(|e| e.to_string())?;
    fs::create_dir_all(&assets_dir).map_err(|e| e.to_string())?;
    fs::create_dir_all(&versions_dir).map_err(|e| e.to_string())?;

    // Initialize HTTP client
    let client = http_client()?;

    // 1. Fetch or load version JSON
    let local_ver_dir = versions_dir.join(version);
    fs::create_dir_all(&local_ver_dir).map_err(|e| e.to_string())?;
    let local_ver_json_path = local_ver_dir.join(format!("{version}.json"));
    let local_ver_jar_path = local_ver_dir.join(format!("{version}.jar"));
    let mut loader_profile_json: Option<String> = None;

    let version_info: VersionInfo = if local_ver_json_path.is_file() {
        let content = fs::read_to_string(&local_ver_json_path)
            .map_err(|e| format!("Failed to read local version JSON: {e}"))?;
        serde_json::from_str(&content)
            .map_err(|e| format!("Failed to parse local version JSON: {e}"))?
    } else {
        let manifest_url = constants::MOJANG_VERSION_MANIFEST_URL;
        let manifest: VersionManifest =
            fetch_json(&client, manifest_url, "version manifest").await?;
        let manifest_ver = manifest
            .versions
            .iter()
            .find(|v| v.id == version)
            .ok_or_else(|| format!("Version {version} not found in manifest or local versions"))?;

        load_or_fetch_json(
            &client,
            &local_ver_json_path,
            &manifest_ver.url,
            "version JSON",
        )
        .await?
    };
    if version_info.id != version {
        return Err("Downloaded version metadata does not match the selected version".to_string());
    }

    download_if_missing_verified(
        &client,
        &version_info.downloads.client.url,
        &local_ver_jar_path,
        version_info.downloads.client.sha1.as_deref(),
        version_info.downloads.client.size,
    )
    .await?;

    // 2. Fetch Loader profile (Fabric / Quilt)
    let loader_profile: Option<LoaderProfile> = match loader {
        constants::FABRIC_LOADER => {
            // Find latest stable fabric loader version for this game version
            let loaders_url = format!(
                "{}/versions/loader/{}",
                constants::FABRIC_META_BASE_URL,
                version
            );
            let loaders: serde_json::Value =
                fetch_json(&client, &loaders_url, "Fabric loader list").await?;

            let loader_ver = loaders
                .as_array()
                .and_then(|arr| {
                    arr.iter()
                        .find(|item| item["loader"]["stable"].as_bool().unwrap_or(false))
                })
                .or_else(|| loaders.as_array().and_then(|arr| arr.first()))
                .and_then(|item| item["loader"]["version"].as_str())
                .ok_or_else(|| format!("No Fabric loader found for version {version}"))?;

            let profile_url = format!(
                "{}/versions/loader/{}/{}/profile/json",
                constants::FABRIC_META_BASE_URL,
                version,
                loader_ver
            );
            let profile_json = client
                .get(&profile_url)
                .header("User-Agent", constants::USER_AGENT)
                .send()
                .await
                .map_err(|e| format!("Failed to fetch Fabric profile: {}", e))?
                .error_for_status()
                .map_err(|e| format!("Fabric profile returned an error: {}", e))?
                .text()
                .await
                .map_err(|e| format!("Failed to read Fabric profile: {}", e))?;

            loader_profile_json = Some(profile_json.clone());
            let profile: LoaderProfile = serde_json::from_str(&profile_json)
                .map_err(|e| format!("Failed to parse Fabric profile: {}", e))?;
            Some(profile)
        }
        constants::QUILT_LOADER => {
            // Find latest quilt loader version for this game version
            let loaders_url = format!(
                "{}/versions/loader/{}",
                constants::QUILT_META_BASE_URL,
                version
            );
            let loaders: serde_json::Value =
                fetch_json(&client, &loaders_url, "Quilt loader list").await?;

            let loader_ver = loaders
                .as_array()
                .and_then(|arr| arr.first())
                .and_then(|item| item["loader"]["version"].as_str())
                .ok_or_else(|| format!("No Quilt loader found for version {version}"))?;

            let profile_url = format!(
                "{}/versions/loader/{}/{}/profile/json",
                constants::QUILT_META_BASE_URL,
                version,
                loader_ver
            );
            let profile_json = client
                .get(&profile_url)
                .header("User-Agent", constants::USER_AGENT)
                .send()
                .await
                .map_err(|e| format!("Failed to fetch Quilt profile: {}", e))?
                .error_for_status()
                .map_err(|e| format!("Quilt profile returned an error: {}", e))?
                .text()
                .await
                .map_err(|e| format!("Failed to read Quilt profile: {}", e))?;

            loader_profile_json = Some(profile_json.clone());
            let profile: LoaderProfile = serde_json::from_str(&profile_json)
                .map_err(|e| format!("Failed to parse Quilt profile: {}", e))?;
            Some(profile)
        }
        constants::VANILLA_LOADER => None,
        _ => {
            return Err(
                "This loader is not supported yet. Choose Vanilla, Fabric, or Quilt.".to_string(),
            );
        }
    };

    // Gather all library download requirements
    let mut libraries_to_download = Vec::new();

    // Vanilla libraries (only those allowed on linux)
    for lib in &version_info.libraries {
        if should_allow_library(lib.rules.as_deref().unwrap_or(&[])) {
            libraries_to_download.push(lib.clone());
        }
    }

    // Loader libraries (only those allowed on linux)
    if let Some(profile) = &loader_profile {
        for lib in &profile.libraries {
            if should_allow_library(lib.rules.as_deref().unwrap_or(&[])) {
                libraries_to_download.push(lib.clone());
            }
        }
    }

    // 3. Download Libraries in parallel
    let client_arc = Arc::new(client.clone());
    let libraries_dir_arc = Arc::new(libraries_dir.clone());

    let mut lib_tasks = tokio::task::JoinSet::new();
    let mut scheduled_library_paths = HashSet::new();
    for lib in libraries_to_download {
        // Collect URLs to download
        let mut downloads = Vec::new();

        // 1. Artifact jar
        if let Some(url) = get_library_url(&lib) {
            if let Some(path) = get_library_path(&lib) {
                let verification = lib
                    .downloads
                    .as_ref()
                    .and_then(|downloads| downloads.artifact.as_ref())
                    .map(|artifact| (artifact.sha1.clone(), artifact.size));
                downloads.push((url, path, verification));
            }
        }

        // 2. Native classifier (if any)
        if let Some(native_art) = get_native_library(&lib) {
            if let Some(path_str) = &native_art.path {
                if let Some(path) = safe_relative_path(path_str) {
                    downloads.push((
                        native_art.url.clone(),
                        path,
                        Some((native_art.sha1.clone(), native_art.size)),
                    ));
                }
            }
        }

        for (url, path, verification) in downloads {
            let cl = client_arc.clone();
            let dest = libraries_dir_arc.join(path);
            if !scheduled_library_paths.insert(dest.clone()) {
                continue;
            }

            if lib_tasks.len() >= constants::DOWNLOAD_CONCURRENCY {
                lib_tasks
                    .join_next()
                    .await
                    .ok_or_else(|| "Library download task disappeared".to_string())?
                    .map_err(|e| format!("Library download task failed: {e}"))??;
            }
            lib_tasks.spawn(async move {
                let (expected_sha1, expected_size) = verification.unwrap_or((None, None));
                download_if_missing_verified(
                    &cl,
                    &url,
                    &dest,
                    expected_sha1.as_deref(),
                    expected_size,
                )
                .await
            });
        }
    }

    while let Some(task) = lib_tasks.join_next().await {
        task.map_err(|e| format!("Library download task failed: {e}"))??;
    }

    // 4. Download Assets
    // Fetch asset index JSON
    let asset_index_id = version_info.asset_index.id.trim();
    if safe_relative_component(asset_index_id).is_none() {
        return Err("Minecraft asset index has an invalid identifier".to_string());
    }
    let asset_index_path = assets_dir
        .join("indexes")
        .join(format!("{asset_index_id}.json"));
    let asset_index: AssetIndex = load_or_fetch_json(
        &client,
        &asset_index_path,
        &version_info.asset_index.url,
        "asset index",
    )
    .await?;

    // Download assets concurrently
    let mut asset_tasks = tokio::task::JoinSet::new();
    let mut scheduled_asset_paths = HashSet::new();
    let objects_dir = assets_dir.join("objects");

    for asset_obj in asset_index.objects.values() {
        let hash = &asset_obj.hash;
        if hash.len() != 40 || !hash.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return Err("Minecraft asset index contained an invalid asset hash".to_string());
        }
        let sub = hash[0..2].to_string();
        let hash = hash.to_string();
        let url = format!("{}/{}/{}", constants::MINECRAFT_ASSET_BASE_URL, sub, hash);
        let dest = objects_dir.join(&sub).join(&hash);
        if !scheduled_asset_paths.insert(dest.clone()) {
            continue;
        }
        let asset_hash = hash.clone();
        let asset_size = asset_obj.size;

        let cl = client_arc.clone();

        if asset_tasks.len() >= constants::DOWNLOAD_CONCURRENCY {
            asset_tasks
                .join_next()
                .await
                .ok_or_else(|| "Asset download task disappeared".to_string())?
                .map_err(|e| format!("Asset download task failed: {e}"))??;
        }
        asset_tasks.spawn(async move {
            download_if_missing_verified(&cl, &url, &dest, Some(&asset_hash), asset_size).await
        });
    }

    while let Some(task) = asset_tasks.join_next().await {
        task.map_err(|e| format!("Asset download task failed: {e}"))??;
    }

    // Java setup
    let required_java = version_info
        .java_version
        .map(|jv| jv.major_version)
        .unwrap_or_else(|| required_java_major(version));

    let mut java_path = request.java_path.clone();
    if java_major(&java_path).unwrap_or(0) < required_java {
        java_path = install_java_runtime(required_java)?;
    }

    // Publish the profile only after every remote dependency has succeeded.
    // A failed download therefore cannot leave a directory that looks valid
    // to the instance scanner.
    fs::create_dir_all(&profile_dir).map_err(|e| e.to_string())?;
    for folder in ["mods", "resourcepacks", "shaderpacks", "logs"] {
        fs::create_dir_all(profile_dir.join(folder)).map_err(|e| e.to_string())?;
    }
    if let Some(profile_json) = loader_profile_json {
        atomic_write(&profile_dir.join("profile.json"), profile_json.as_bytes())
            .map_err(|e| e.to_string())?;
    }

    let (existing_java, existing_memory) = read_existing_overrides(&profile_dir);
    mod_write_profile(
        &profile_dir,
        &request.name,
        version,
        loader,
        &request.player_name,
        &java_path,
        request.memory_gb,
        existing_java.as_deref(),
        existing_memory,
    )
    .map_err(|error| error.to_string())?;

    Ok(request.name)
}

pub async fn download_if_missing_verified(
    client: &reqwest::Client,
    url: &str,
    path: &Path,
    expected_sha1: Option<&str>,
    expected_size: Option<u64>,
) -> Result<(), String> {
    validate_remote_url(url)?;
    if path.is_file() && verify_file(path, expected_sha1, expected_size) {
        return Ok(());
    }
    if path.exists() {
        let _ = fs::remove_file(path);
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }

    let partial = unique_temp_path(path, "part");
    let mut last_error = String::from("download failed");
    for attempt in 1..=constants::DOWNLOAD_ATTEMPTS {
        let _ = fs::remove_file(&partial);
        let response = match client
            .get(url)
            .header("User-Agent", constants::USER_AGENT)
            .send()
            .await
        {
            Ok(response) if response.status().is_success() => response,
            Ok(response) => {
                last_error = format!("HTTP {}", response.status());
                if response.status().is_client_error() {
                    break;
                }
                if attempt < constants::DOWNLOAD_ATTEMPTS {
                    tokio::time::sleep(Duration::from_millis(
                        constants::DOWNLOAD_RETRY_DELAY_MILLIS * u64::from(attempt),
                    ))
                    .await;
                }
                continue;
            }
            Err(error) => {
                last_error = format!("request failed: {error}");
                if attempt < constants::DOWNLOAD_ATTEMPTS {
                    tokio::time::sleep(Duration::from_millis(
                        constants::DOWNLOAD_RETRY_DELAY_MILLIS * u64::from(attempt),
                    ))
                    .await;
                }
                continue;
            }
        };

        let write_result = async {
            let mut file = tokio::fs::File::create(&partial)
                .await
                .map_err(|error| format!("could not create partial file: {error}"))?;
            let mut response = response;
            use tokio::io::AsyncWriteExt;
            while let Some(chunk) = response
                .chunk()
                .await
                .map_err(|error| format!("failed while reading download: {error}"))?
            {
                file.write_all(&chunk)
                    .await
                    .map_err(|error| format!("could not write download: {error}"))?;
            }
            file.sync_all()
                .await
                .map_err(|error| format!("could not flush download: {error}"))?;
            Ok::<(), String>(())
        }
        .await;

        if let Err(error) = write_result {
            last_error = error;
            let _ = fs::remove_file(&partial);
            if attempt < constants::DOWNLOAD_ATTEMPTS {
                tokio::time::sleep(Duration::from_millis(
                    constants::DOWNLOAD_RETRY_DELAY_MILLIS * u64::from(attempt),
                ))
                .await;
            }
            continue;
        }

        if !verify_file(&partial, expected_sha1, expected_size) {
            last_error = "downloaded file failed size/hash verification".to_string();
            let _ = fs::remove_file(&partial);
            if attempt < constants::DOWNLOAD_ATTEMPTS {
                tokio::time::sleep(Duration::from_millis(
                    constants::DOWNLOAD_RETRY_DELAY_MILLIS * u64::from(attempt),
                ))
                .await;
            }
            continue;
        }

        publish_replacement(&partial, path)
            .map_err(|error| format!("could not publish download: {error}"))?;
        return Ok(());
    }

    let _ = fs::remove_file(&partial);
    Err(format!("{}: {}", last_error, url))
}

fn verify_file(path: &Path, expected_sha1: Option<&str>, expected_size: Option<u64>) -> bool {
    let Ok(metadata) = fs::metadata(path) else {
        return false;
    };
    if metadata.len() == 0 || expected_size.is_some_and(|size| size != metadata.len()) {
        return false;
    }
    let Some(expected_sha1) = expected_sha1 else {
        return true;
    };
    let Ok(file) = fs::File::open(path) else {
        return false;
    };
    use sha1::{Digest, Sha1};
    let mut reader = std::io::BufReader::new(file);
    let mut hasher = Sha1::new();
    if std::io::copy(&mut reader, &mut hasher).is_err() {
        return false;
    }
    format!("{:x}", hasher.finalize()).eq_ignore_ascii_case(expected_sha1)
}

fn http_client() -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(constants::DOWNLOAD_TIMEOUT_SECONDS))
        .build()
        .map_err(|error| format!("could not initialize HTTP client: {error}"))
}

#[cfg(test)]
mod tests {
    use super::{is_valid_instance_name, validated_version_id};

    #[test]
    fn instance_names_and_versions_are_path_safe() {
        assert!(is_valid_instance_name("My Survival"));
        assert!(!is_valid_instance_name("../outside"));
        assert!(!is_valid_instance_name(""));
        assert_eq!(validated_version_id(" 1.21.8 ").unwrap(), "1.21.8");
        assert!(validated_version_id("../latest").is_err());
        assert!(validated_version_id("1.21/other").is_err());
    }
}

async fn fetch_json<T: DeserializeOwned>(
    client: &reqwest::Client,
    url: &str,
    label: &str,
) -> Result<T, String> {
    validate_remote_url(url)?;
    client
        .get(url)
        .header("User-Agent", constants::USER_AGENT)
        .send()
        .await
        .map_err(|error| format!("Failed to fetch {label}: {error}"))?
        .error_for_status()
        .map_err(|error| format!("{label} returned an error: {error}"))?
        .json()
        .await
        .map_err(|error| format!("Failed to parse {label}: {error}"))
}

async fn load_or_fetch_json<T: DeserializeOwned>(
    client: &reqwest::Client,
    cache_path: &Path,
    url: &str,
    label: &str,
) -> Result<T, String> {
    validate_remote_url(url)?;
    if cache_path.is_file() {
        if let Ok(bytes) = fs::read(cache_path) {
            if let Ok(value) = serde_json::from_slice(&bytes) {
                return Ok(value);
            }
        }
        // A truncated/old cache must not permanently block installation.
        let _ = fs::remove_file(cache_path);
    }

    let bytes = client
        .get(url)
        .header("User-Agent", constants::USER_AGENT)
        .send()
        .await
        .map_err(|error| format!("Failed to fetch {label}: {error}"))?
        .error_for_status()
        .map_err(|error| format!("{label} returned an error: {error}"))?
        .bytes()
        .await
        .map_err(|error| format!("Failed to read {label}: {error}"))?;
    let value = serde_json::from_slice(&bytes)
        .map_err(|error| format!("Failed to parse {label}: {error}"))?;
    atomic_write(cache_path, &bytes)
        .map_err(|error| format!("Could not cache {label}: {error}"))?;
    Ok(value)
}

pub(crate) fn is_valid_instance_name(name: &str) -> bool {
    let trimmed = name.trim();
    !trimmed.is_empty()
        && trimmed.chars().count() <= constants::MAX_INSTANCE_NAME_CHARS
        && trimmed.chars().all(|character| {
            !character.is_control() && character != '/' && character != '\\' && character != '\0'
        })
        && safe_file_name(&sanitize_name(trimmed)).is_some()
}

fn validated_version_id(value: &str) -> Result<&str, String> {
    let version = value.trim();
    if version.is_empty()
        || version.chars().count() > 64
        || safe_relative_component(version).is_none()
    {
        return Err("Minecraft version has an invalid identifier".to_string());
    }
    Ok(version)
}

pub async fn prepare_instance_task(request: CreateRequest) -> Result<String, String> {
    if !is_valid_instance_name(&request.name) {
        return Err(
            "Instance name must contain 1–64 visible characters and no path separators".to_string(),
        );
    }
    let loader = constants::canonical_loader(&request.loader).ok_or_else(|| {
        "This loader is not supported yet. Choose Vanilla, Fabric, or Quilt.".to_string()
    })?;
    let version = validated_version_id(&request.version)?;
    if request.root.trim().is_empty() {
        return Err("Game directory cannot be empty".to_string());
    }
    let game_root = PathBuf::from(request.root.trim());
    let dot_rixlauncher = game_root.parent().unwrap_or(&game_root);
    let profile_dir = game_root.join(sanitize_name(&request.name));

    let libraries_dir = dot_rixlauncher.join("libraries");
    let assets_dir = dot_rixlauncher.join("assets");
    let versions_dir = dot_rixlauncher.join("versions");
    fs::create_dir_all(&libraries_dir).map_err(|e| e.to_string())?;
    fs::create_dir_all(&assets_dir).map_err(|e| e.to_string())?;
    fs::create_dir_all(&versions_dir).map_err(|e| e.to_string())?;

    let client = http_client()?;

    // Fetch or load version JSON
    let local_ver_dir = versions_dir.join(version);
    fs::create_dir_all(&local_ver_dir).map_err(|e| e.to_string())?;
    let local_ver_json_path = local_ver_dir.join(format!("{version}.json"));
    let local_ver_jar_path = local_ver_dir.join(format!("{version}.jar"));
    let mut loader_profile_json: Option<String> = None;

    let version_info: VersionInfo = if local_ver_json_path.is_file() {
        let content = fs::read_to_string(&local_ver_json_path)
            .map_err(|e| format!("Failed to read local version JSON: {e}"))?;
        serde_json::from_str(&content)
            .map_err(|e| format!("Failed to parse local version JSON: {e}"))?
    } else {
        let manifest_url = constants::MOJANG_VERSION_MANIFEST_URL;
        let manifest: VersionManifest =
            fetch_json(&client, manifest_url, "version manifest").await?;
        let manifest_ver = manifest
            .versions
            .iter()
            .find(|v| v.id == version)
            .ok_or_else(|| format!("Version {version} not found in manifest or local versions"))?;

        load_or_fetch_json(
            &client,
            &local_ver_json_path,
            &manifest_ver.url,
            "version JSON",
        )
        .await?
    };
    if version_info.id != version {
        return Err("Downloaded version metadata does not match the selected version".to_string());
    }

    download_if_missing_verified(
        &client,
        &version_info.downloads.client.url,
        &local_ver_jar_path,
        version_info.downloads.client.sha1.as_deref(),
        version_info.downloads.client.size,
    )
    .await?;

    let loader_profile: Option<LoaderProfile> = match loader {
        constants::FABRIC_LOADER => {
            let loaders_url = format!(
                "{}/versions/loader/{}",
                constants::FABRIC_META_BASE_URL,
                version
            );
            let loaders: serde_json::Value =
                fetch_json(&client, &loaders_url, "Fabric loader list").await?;

            let loader_ver = loaders
                .as_array()
                .and_then(|arr| {
                    arr.iter()
                        .find(|item| item["loader"]["stable"].as_bool().unwrap_or(false))
                })
                .or_else(|| loaders.as_array().and_then(|arr| arr.first()))
                .and_then(|item| item["loader"]["version"].as_str())
                .ok_or_else(|| format!("No Fabric loader found for version {version}"))?;

            let profile_url = format!(
                "{}/versions/loader/{}/{}/profile/json",
                constants::FABRIC_META_BASE_URL,
                version,
                loader_ver
            );
            let profile_json = client
                .get(&profile_url)
                .header("User-Agent", constants::USER_AGENT)
                .send()
                .await
                .map_err(|e| format!("Failed to fetch Fabric profile: {}", e))?
                .error_for_status()
                .map_err(|e| format!("Fabric profile returned an error: {}", e))?
                .text()
                .await
                .map_err(|e| format!("Failed to read Fabric profile: {}", e))?;

            loader_profile_json = Some(profile_json.clone());
            let profile: LoaderProfile = serde_json::from_str(&profile_json)
                .map_err(|e| format!("Failed to parse Fabric profile: {}", e))?;
            Some(profile)
        }
        constants::QUILT_LOADER => {
            let loaders_url = format!(
                "{}/versions/loader/{}",
                constants::QUILT_META_BASE_URL,
                version
            );
            let loaders: serde_json::Value =
                fetch_json(&client, &loaders_url, "Quilt loader list").await?;

            let loader_ver = loaders
                .as_array()
                .and_then(|arr| arr.first())
                .and_then(|item| item["loader"]["version"].as_str())
                .ok_or_else(|| format!("No Quilt loader found for version {version}"))?;

            let profile_url = format!(
                "{}/versions/loader/{}/{}/profile/json",
                constants::QUILT_META_BASE_URL,
                version,
                loader_ver
            );
            let profile_json = client
                .get(&profile_url)
                .header("User-Agent", constants::USER_AGENT)
                .send()
                .await
                .map_err(|e| format!("Failed to fetch Quilt profile: {}", e))?
                .error_for_status()
                .map_err(|e| format!("Quilt profile returned an error: {}", e))?
                .text()
                .await
                .map_err(|e| format!("Failed to read Quilt profile: {}", e))?;

            loader_profile_json = Some(profile_json.clone());
            let profile: LoaderProfile = serde_json::from_str(&profile_json)
                .map_err(|e| format!("Failed to parse Quilt profile: {}", e))?;
            Some(profile)
        }
        constants::VANILLA_LOADER => {
            // A Vanilla preparation must not retain a stale loader profile.
            if profile_dir.is_dir() {
                let _ = fs::remove_file(profile_dir.join("profile.json"));
            }
            None
        }
        _ => {
            return Err(
                "This loader is not supported yet. Choose Vanilla, Fabric, or Quilt.".to_string(),
            );
        }
    };

    let mut libraries_to_download = Vec::new();
    for lib in &version_info.libraries {
        if should_allow_library(lib.rules.as_deref().unwrap_or(&[])) {
            libraries_to_download.push(lib.clone());
        }
    }
    if let Some(profile) = &loader_profile {
        for lib in &profile.libraries {
            if should_allow_library(lib.rules.as_deref().unwrap_or(&[])) {
                libraries_to_download.push(lib.clone());
            }
        }
    }

    let client_arc = Arc::new(client.clone());
    let libraries_dir_arc = Arc::new(libraries_dir.clone());

    let mut lib_tasks = tokio::task::JoinSet::new();
    let mut scheduled_library_paths = HashSet::new();
    for lib in libraries_to_download {
        let mut downloads = Vec::new();
        if let Some(url) = get_library_url(&lib) {
            if let Some(path) = get_library_path(&lib) {
                let verification = lib
                    .downloads
                    .as_ref()
                    .and_then(|downloads| downloads.artifact.as_ref())
                    .map(|artifact| (artifact.sha1.clone(), artifact.size));
                downloads.push((url, path, verification));
            }
        }
        if let Some(native_art) = get_native_library(&lib) {
            if let Some(path_str) = &native_art.path {
                if let Some(path) = safe_relative_path(path_str) {
                    downloads.push((
                        native_art.url.clone(),
                        path,
                        Some((native_art.sha1.clone(), native_art.size)),
                    ));
                }
            }
        }
        for (url, path, verification) in downloads {
            let cl = client_arc.clone();
            let dest = libraries_dir_arc.join(path);
            if !scheduled_library_paths.insert(dest.clone()) {
                continue;
            }
            if lib_tasks.len() >= constants::DOWNLOAD_CONCURRENCY {
                lib_tasks
                    .join_next()
                    .await
                    .ok_or_else(|| "Library download task disappeared".to_string())?
                    .map_err(|e| format!("Library download task failed: {e}"))??;
            }
            lib_tasks.spawn(async move {
                let (expected_sha1, expected_size) = verification.unwrap_or((None, None));
                download_if_missing_verified(
                    &cl,
                    &url,
                    &dest,
                    expected_sha1.as_deref(),
                    expected_size,
                )
                .await
            });
        }
    }

    while let Some(task) = lib_tasks.join_next().await {
        task.map_err(|e| format!("Library download task failed: {e}"))??;
    }

    let asset_index_id = version_info.asset_index.id.trim();
    if safe_relative_component(asset_index_id).is_none() {
        return Err("Minecraft asset index has an invalid identifier".to_string());
    }
    let asset_index_path = assets_dir
        .join("indexes")
        .join(format!("{asset_index_id}.json"));
    let asset_index: AssetIndex = load_or_fetch_json(
        &client,
        &asset_index_path,
        &version_info.asset_index.url,
        "asset index",
    )
    .await?;

    let mut asset_tasks = tokio::task::JoinSet::new();
    let mut scheduled_asset_paths = HashSet::new();
    let objects_dir = assets_dir.join("objects");

    for (_name, asset) in asset_index.objects {
        let hash = asset.hash.as_str();
        if hash.len() != 40 || !hash.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return Err("Minecraft asset index contained an invalid asset hash".to_string());
        }
        let hash = hash.to_string();
        let asset_size = asset.size;
        let prefix = hash[0..2].to_string();
        let asset_url = format!(
            "{}/{}/{}",
            constants::MINECRAFT_ASSET_BASE_URL,
            prefix,
            hash
        );
        let dest = objects_dir.join(&prefix).join(&hash);
        if !scheduled_asset_paths.insert(dest.clone()) {
            continue;
        }
        let asset_hash = hash.clone();

        let cl = client_arc.clone();
        if asset_tasks.len() >= constants::DOWNLOAD_CONCURRENCY {
            asset_tasks
                .join_next()
                .await
                .ok_or_else(|| "Asset download task disappeared".to_string())?
                .map_err(|e| format!("Asset download task failed: {e}"))??;
        }
        asset_tasks.spawn(async move {
            download_if_missing_verified(&cl, &asset_url, &dest, Some(&asset_hash), asset_size)
                .await
        });
    }

    while let Some(task) = asset_tasks.join_next().await {
        task.map_err(|e| format!("Asset download task failed: {e}"))??;
    }

    let required_java = version_info
        .java_version
        .map(|jv| jv.major_version)
        .unwrap_or_else(|| required_java_major(version));

    let mut java_path = request.java_path.clone();
    if java_major(&java_path).unwrap_or(0) < required_java {
        java_path = install_java_runtime(required_java)?;
    }

    fs::create_dir_all(&profile_dir).map_err(|e| e.to_string())?;
    for folder in ["mods", "resourcepacks", "shaderpacks", "logs"] {
        fs::create_dir_all(profile_dir.join(folder)).map_err(|e| e.to_string())?;
    }
    if let Some(profile_json) = loader_profile_json {
        atomic_write(&profile_dir.join("profile.json"), profile_json.as_bytes())
            .map_err(|e| e.to_string())?;
    }

    let (existing_java, existing_memory) = read_existing_overrides(&profile_dir);
    mod_write_profile(
        &profile_dir,
        &request.name,
        version,
        loader,
        &request.player_name,
        &java_path,
        request.memory_gb,
        existing_java.as_deref(),
        existing_memory,
    )
    .map_err(|error| error.to_string())?;

    Ok(request.name)
}
