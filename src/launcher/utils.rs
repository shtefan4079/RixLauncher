#![allow(dead_code)]
use crate::launcher::constants;
use crate::launcher::types::{ContentItem, LibraryArtifact, LibraryInfo, MinecraftInstance, Rule};
use std::collections::HashMap;
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

pub const USER_AGENT: &str = constants::USER_AGENT;

static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

pub fn unique_temp_path(path: &Path, label: &str) -> PathBuf {
    let sequence = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
    let pid = std::process::id();
    let suffix = format!("{label}-{pid}-{sequence}");
    path.with_extension(suffix)
}

pub fn should_allow_library(rules: &[Rule]) -> bool {
    if rules.is_empty() {
        return true;
    }
    // Mojang rule evaluation starts with `allowed = false`. A matching allow
    // or disallow rule flips the state. If no rule matches, the item is not allowed.
    let mut allowed = false;
    for rule in rules {
        let current_os = if cfg!(target_os = "windows") {
            "windows"
        } else if cfg!(target_os = "macos") {
            "osx"
        } else {
            "linux"
        };
        let matches_os = match &rule.os {
            Some(os) => {
                let matches_name = os
                    .name
                    .as_deref()
                    .is_none_or(|name| name.eq_ignore_ascii_case(current_os));
                let matches_arch = os.arch.as_deref().is_none_or(architecture_matches_current);
                matches_name && matches_arch
            }
            None => true,
        };

        let matches_features = match &rule.features {
            Some(features) => features.iter().all(|(_name, &val)| !val),
            None => true,
        };

        if matches_os && matches_features {
            allowed = rule.action == "allow";
        }
    }
    allowed
}

pub fn get_library_url(lib: &LibraryInfo) -> Option<String> {
    if let Some(downloads) = &lib.downloads {
        if let Some(artifact) = &downloads.artifact {
            return validate_remote_url(&artifact.url)
                .ok()
                .map(|_| artifact.url.clone());
        }
    }

    // Construct maven URL if name and base URL are present
    if let Some(base_url) = &lib.url {
        if let Some((url, _)) = maven_to_url(&lib.name, base_url) {
            return validate_remote_url(&url).ok().map(|_| url);
        }
    } else {
        // Default to Mojang library repo
        if let Some((url, _)) = maven_to_url(&lib.name, constants::MOJANG_LIBRARY_BASE_URL) {
            return validate_remote_url(&url).ok().map(|_| url);
        }
    }
    None
}

pub fn get_library_path(lib: &LibraryInfo) -> Option<PathBuf> {
    if let Some(downloads) = &lib.downloads {
        if let Some(artifact) = &downloads.artifact {
            if let Some(path) = &artifact.path {
                return safe_relative_path(path);
            }
        }
    }

    // Construct path from maven coordinates
    if let Some((_, path)) = maven_to_url(&lib.name, "") {
        return safe_relative_path(&path);
    }
    None
}

pub fn get_native_library(lib: &LibraryInfo) -> Option<LibraryArtifact> {
    let current_os = if cfg!(target_os = "windows") {
        "windows"
    } else if cfg!(target_os = "macos") {
        "osx"
    } else {
        "linux"
    };
    let native_classifier = lib.natives.as_ref()?.get(current_os)?;
    let downloads = lib.downloads.as_ref()?;
    let classifiers = downloads.classifiers.as_ref()?;
    classifiers
        .get(native_classifier)
        .or_else(|| {
            let expanded = native_classifier.replace("${arch}", std::env::consts::ARCH);
            classifiers.get(&expanded)
        })
        .cloned()
}

fn architecture_matches_current(required: &str) -> bool {
    let required = required.trim().to_ascii_lowercase();
    let current = std::env::consts::ARCH.to_ascii_lowercase();
    required == current
        || matches!(
            (required.as_str(), current.as_str()),
            ("amd64", "x86_64") | ("x86-64", "x86_64") | ("arm64", "aarch64") | ("x86", "i686")
        )
}

pub fn maven_to_url(name: &str, base_url: &str) -> Option<(String, String)> {
    let parts: Vec<&str> = name.split(':').collect();
    if parts.len() < 3 {
        return None;
    }
    let group = parts[0].replace('.', "/");
    let artifact = parts[1];
    let version = parts[2];

    let classifier = if parts.len() >= 4 {
        format!("-{}", parts[3])
    } else {
        "".to_string()
    };

    let filename = format!("{}-{}{}.jar", artifact, version, classifier);
    let path = format!("{}/{}/{}/{}", group, artifact, version, filename);
    let download_url = if base_url.is_empty() {
        "".to_string()
    } else if base_url.ends_with('/') {
        format!("{}{}", base_url, path)
    } else {
        format!("{}/{}", base_url, path)
    };
    safe_relative_path(&path).map(|_| (download_url, path))
}

/// Accept only relative paths made of normal components. Download metadata is
/// remote input, so a path such as `../../config.json` must never escape the
/// launcher's cache directory. Backslashes are rejected too so the check is
/// safe when the same metadata is used on Windows.
pub fn safe_relative_path(value: &str) -> Option<PathBuf> {
    if value.is_empty() || value.contains(['\\', ':']) {
        return None;
    }
    let path = Path::new(value);
    let mut has_component = false;
    for component in path.components() {
        if !matches!(component, Component::Normal(_)) {
            return None;
        }
        has_component = true;
    }
    has_component.then(|| path.to_path_buf())
}

/// Accept one portable file-name component and reject every value that could
/// be interpreted as a path, device name, or shell/control input. File names
/// can arrive from manifests, imported archives, or UI messages, so callers
/// should validate before joining them to an instance directory.
pub fn safe_file_name(value: &str) -> Option<&str> {
    if value.is_empty()
        || value == "."
        || value == ".."
        || value.contains(['/', '\\', ':'])
        || value.chars().any(|character| character.is_control())
        || value.len() > 255
    {
        return None;
    }

    #[cfg(windows)]
    {
        if value.ends_with(['.', ' ']) {
            return None;
        }
        let stem = value.split('.').next().unwrap_or_default();
        if matches!(
            stem.to_ascii_uppercase().as_str(),
            "CON"
                | "PRN"
                | "AUX"
                | "NUL"
                | "COM1"
                | "COM2"
                | "COM3"
                | "COM4"
                | "COM5"
                | "COM6"
                | "COM7"
                | "COM8"
                | "COM9"
                | "LPT1"
                | "LPT2"
                | "LPT3"
                | "LPT4"
                | "LPT5"
                | "LPT6"
                | "LPT7"
                | "LPT8"
                | "LPT9"
        ) {
            return None;
        }
    }

    Some(value)
}

/// Accept a single relative path component, which is useful for version IDs,
/// asset indexes, and other metadata that must become one directory/file
/// name rather than a nested path.
pub fn safe_relative_component(value: &str) -> Option<PathBuf> {
    let path = safe_relative_path(value)?;
    (path.components().count() == 1).then_some(path)
}

/// Returns the enabled file name represented by a disabled mod file.
/// The suffix is intentionally case-insensitive because users can receive
/// files from Windows tools that preserve an uppercase extension.
pub fn strip_disabled_suffix(value: &str) -> Option<&str> {
    let suffix = ".disabled";
    let start = value.len().checked_sub(suffix.len())?;
    let tail = value.get(start..)?;
    tail.eq_ignore_ascii_case(suffix).then(|| &value[..start])
}

pub fn is_disabled_mod_filename(value: &str) -> bool {
    strip_disabled_suffix(value).is_some()
}

/// Remote metadata is only allowed to address HTTPS resources. Apart from
/// making failures clearer, this prevents a malformed manifest from turning
/// the curl fallback into a local-file read.
pub fn validate_remote_url(value: &str) -> Result<(), String> {
    let parsed = url::Url::parse(value).map_err(|error| format!("invalid remote URL: {error}"))?;
    if parsed.scheme() != "https" || parsed.host_str().is_none() {
        return Err("remote URL must use HTTPS and include a host".to_string());
    }
    Ok(())
}

pub fn replace_variables(arg: &str, vars: &HashMap<&str, String>) -> String {
    let mut result = arg.to_string();
    for (k, v) in vars {
        let placeholder = format!("${{{}}}", k);
        result = result.replace(&placeholder, v);
    }
    result
}

pub fn sanitize_name(value: &str) -> String {
    let mut sanitized = String::with_capacity(value.len().min(constants::MAX_INSTANCE_NAME_CHARS));
    let mut previous_separator = false;
    for ch in value.chars().filter(|ch| !ch.is_control()) {
        let allowed = ch.is_alphanumeric() || matches!(ch, '-' | '_');
        if allowed {
            sanitized.push(ch);
            previous_separator = false;
        } else if !previous_separator {
            sanitized.push('_');
            previous_separator = true;
        }
        if sanitized.chars().count() >= constants::MAX_INSTANCE_NAME_CHARS {
            break;
        }
    }
    let trimmed = sanitized.trim_matches(['_', '-']).to_string();
    if trimmed.is_empty() {
        "minecraft_instance".to_string()
    } else {
        trimmed
    }
}

/// Returns a deterministic identifier derived from the profile path. Unlike
/// an array index, it remains attached to the same instance after sorting or
/// after another profile is added/removed.
pub fn stable_instance_id(path: &Path) -> usize {
    use sha1::{Digest, Sha1};

    let identity = fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    let digest = Sha1::digest(identity.to_string_lossy().as_bytes());
    let byte_len = std::mem::size_of::<usize>();
    let mut bytes = [0_u8; std::mem::size_of::<usize>()];
    bytes.copy_from_slice(&digest[..byte_len]);
    usize::from_ne_bytes(bytes)
}

pub fn offline_uuid(name: &str) -> String {
    // Match Minecraft's standard offline UUID algorithm: UUID v3 from the
    // UTF-8 bytes of "OfflinePlayer:<name>". This is deterministic, stable
    // across restarts, and does not collapse different names to a length-based
    // identifier.
    use md5::{Digest, Md5};
    let mut digest: [u8; 16] = Md5::digest(format!("OfflinePlayer:{name}").as_bytes()).into();
    digest[6] = (digest[6] & 0x0f) | 0x30;
    digest[8] = (digest[8] & 0x3f) | 0x80;
    let hex = digest
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    format!(
        "{}-{}-{}-{}-{}",
        &hex[0..8],
        &hex[8..12],
        &hex[12..16],
        &hex[16..20],
        &hex[20..32]
    )
}

/// Replaces a file atomically so a crash cannot leave a half-written JSON or
/// profile file in place.
pub fn atomic_write(path: &Path, contents: &[u8]) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let temporary = unique_temp_path(path, "tmp");
    {
        use std::io::Write;
        let mut file = fs::File::create(&temporary)?;
        file.write_all(contents)?;
        file.sync_all()?;
    }
    publish_replacement(&temporary, path)
}

/// Copies a file through a temporary sibling and publishes it as one logical
/// replacement. This keeps imports, clones and installed assets from exposing
/// partially copied files to the launcher or the game.
pub fn atomic_copy_file(source: &Path, destination: &Path) -> std::io::Result<()> {
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent)?;
    }
    let temporary = unique_temp_path(destination, "copy");
    let result =
        fs::copy(source, &temporary).and_then(|_| publish_replacement(&temporary, destination));
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result.map(|_| ())
}

/// Appends diagnostic text while keeping one rotated backup beside the active
/// log. A panic or failed startup should add context instead of erasing the
/// previous failure report.
pub fn append_rotating_log(path: &Path, contents: &[u8], max_bytes: u64) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let current_size = fs::metadata(path)
        .map(|metadata| metadata.len())
        .unwrap_or(0);
    if current_size > 0 && current_size.saturating_add(contents.len() as u64) > max_bytes {
        let rotated = path.with_extension("log.1");
        let _ = fs::remove_file(&rotated);
        fs::rename(path, rotated)?;
    }
    use std::io::Write;
    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;
    file.write_all(contents)?;
    file.sync_all()
}

pub(crate) fn publish_replacement(temporary: &Path, destination: &Path) -> std::io::Result<()> {
    match fs::rename(temporary, destination) {
        Ok(()) => Ok(()),
        Err(error) if cfg!(target_os = "windows") && destination.exists() => {
            fs::remove_file(destination)?;
            fs::rename(temporary, destination).map_err(|_| error)
        }
        Err(error) => Err(error),
    }
}

pub fn display_name_from_path(path: &Path) -> String {
    path.file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("Minecraft Instance")
        .replace('_', " ")
}

pub fn count_jars(path: &Path) -> usize {
    let Ok(entries) = fs::read_dir(path) else {
        return 0;
    };
    entries
        .flatten()
        .filter(|entry| {
            entry
                .path()
                .extension()
                .and_then(|extension| extension.to_str())
                .is_some_and(|extension| extension.eq_ignore_ascii_case("jar"))
        })
        .count()
}

pub fn count_files(path: &Path) -> usize {
    fs::read_dir(path)
        .map(|entries| {
            entries
                .flatten()
                .filter(|entry| {
                    entry
                        .file_type()
                        .map(|kind| kind.is_file())
                        .unwrap_or(false)
                })
                .count()
        })
        .unwrap_or(0)
}

pub fn instance_icon_text(instance: &MinecraftInstance) -> &'static str {
    match instance.loader.as_str() {
        constants::VANILLA_LOADER => "V",
        "Forge" => "F",
        constants::QUILT_LOADER => "Q",
        _ => "M",
    }
}

pub fn player_initials(player_name: &str) -> String {
    let initials: String = player_name
        .split_whitespace()
        .filter_map(|part| part.chars().next())
        .take(2)
        .flat_map(|ch| ch.to_uppercase())
        .collect();
    if initials.is_empty() {
        "P".to_string()
    } else {
        initials
    }
}

pub fn url_encode(query: &str) -> String {
    let mut encoded = String::new();
    for byte in query.bytes() {
        if byte.is_ascii_alphanumeric()
            || byte == b'-'
            || byte == b'_'
            || byte == b'.'
            || byte == b'~'
        {
            encoded.push(byte as char);
        } else {
            encoded.push_str(&format!("%{:02X}", byte));
        }
    }
    encoded
}

pub fn curl_get(url: &str, headers: &[&str]) -> Result<String, String> {
    validate_remote_url(url)?;
    let mut command = Command::new("curl");
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(constants::WINDOWS_CREATE_NO_WINDOW);
    }
    command
        .arg("-g")
        .arg("-L")
        .arg("-f")
        .arg("-sS")
        .arg("--connect-timeout")
        .arg(constants::CURL_CONNECT_TIMEOUT_SECONDS.to_string())
        .arg("--max-time")
        .arg(constants::SHORT_REQUEST_TIMEOUT_SECONDS.to_string())
        .arg("--retry")
        .arg("2")
        .arg("--retry-delay")
        .arg("1")
        .arg("-H")
        .arg(format!("User-Agent: {}", USER_AGENT));
    for header in headers {
        command.arg("-H").arg(header);
    }
    let output = command
        .arg(url)
        .output()
        .map_err(|error| format!("curl failed: {error}"))?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).trim().to_string());
    }
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

pub fn curl_get_bytes(url: &str) -> Result<Vec<u8>, String> {
    validate_remote_url(url)?;
    let mut command = Command::new("curl");
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(constants::WINDOWS_CREATE_NO_WINDOW);
    }
    let output = command
        .arg("-g")
        .arg("-L")
        .arg("-f")
        .arg("-sS")
        .arg("--connect-timeout")
        .arg(constants::CURL_CONNECT_TIMEOUT_SECONDS.to_string())
        .arg("--max-time")
        .arg(constants::SHORT_REQUEST_TIMEOUT_SECONDS.to_string())
        .arg("--retry")
        .arg("2")
        .arg("--retry-delay")
        .arg("1")
        .arg("-H")
        .arg(format!("User-Agent: {}", USER_AGENT))
        .arg(url)
        .output()
        .map_err(|error| format!("curl failed: {error}"))?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).trim().to_string());
    }
    Ok(output.stdout)
}

pub fn curl_download(url: &str, target: &Path, headers: &[&str]) -> Result<(), String> {
    validate_remote_url(url)?;
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    let partial = unique_temp_path(target, "part");
    let _ = fs::remove_file(&partial);
    let mut command = Command::new("curl");
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(constants::WINDOWS_CREATE_NO_WINDOW);
    }
    command
        .arg("-g")
        .arg("-L")
        .arg("-f")
        .arg("-sS")
        .arg("--connect-timeout")
        .arg(constants::CURL_CONNECT_TIMEOUT_SECONDS.to_string())
        .arg("--max-time")
        .arg(constants::DOWNLOAD_TIMEOUT_SECONDS.to_string())
        .arg("--retry")
        .arg("2")
        .arg("--retry-delay")
        .arg("1")
        .arg("-H")
        .arg(format!("User-Agent: {}", USER_AGENT));
    for header in headers {
        command.arg("-H").arg(header);
    }
    let output = command
        .arg("-o")
        .arg(&partial)
        .arg(url)
        .output()
        .map_err(|error| format!("curl failed: {error}"))?;
    if output.status.success() {
        publish_replacement(&partial, target)
            .map_err(|error| format!("could not publish download: {error}"))
    } else {
        let _ = fs::remove_file(&partial);
        Err(String::from_utf8_lossy(&output.stderr).trim().to_string())
    }
}

#[derive(serde::Deserialize)]
struct ModrinthSearchResponse {
    hits: Vec<ModrinthSearchHit>,
}

#[derive(serde::Deserialize)]
struct ModrinthSearchHit {
    project_id: String,
    #[serde(default)]
    slug: String,
    title: String,
    #[serde(default)]
    description: String,
    #[serde(default)]
    project_type: String,
    icon_url: Option<String>,
}

pub fn parse_modrinth_hits(json: &str) -> Result<Vec<ContentItem>, String> {
    let response: ModrinthSearchResponse = serde_json::from_str(json)
        .map_err(|error| format!("Could not parse Modrinth search response: {error}"))?;
    Ok(response
        .hits
        .into_iter()
        .map(|hit| ContentItem {
            id: hit.project_id,
            slug: hit.slug,
            title: hit.title,
            description: hit.description,
            project_type: hit.project_type,
            icon_url: hit.icon_url,
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn safe_paths_reject_escape_attempts() {
        assert!(safe_relative_path("com/example/library.jar").is_some());
        assert!(safe_relative_component("1.21.8").is_some());
        assert!(safe_file_name("library.jar").is_some());
        assert!(safe_relative_path("../config.json").is_none());
        assert!(safe_relative_path("/tmp/config.json").is_none());
        assert!(safe_relative_path(r"..\config.json").is_none());
        assert!(safe_relative_component("version/other").is_none());
        assert!(safe_file_name("../config.json").is_none());
        assert!(safe_file_name("library.jar\nother").is_none());
    }

    #[test]
    fn disabled_mod_suffix_is_case_insensitive_and_utf8_safe() {
        assert_eq!(
            strip_disabled_suffix("sodium.jar.disabled"),
            Some("sodium.jar")
        );
        assert_eq!(
            strip_disabled_suffix("sodium.jar.DISABLED"),
            Some("sodium.jar")
        );
        assert_eq!(
            strip_disabled_suffix("unicode_mod_ünicode.jar.DISABLED"),
            Some("unicode_mod_ünicode.jar")
        );
        assert_eq!(strip_disabled_suffix("sodium.jar"), None);
    }

    #[test]
    fn remote_urls_require_https() {
        assert!(validate_remote_url("https://example.com/file.jar").is_ok());
        assert!(validate_remote_url("http://example.com/file.jar").is_err());
        assert!(validate_remote_url("file:///tmp/file.jar").is_err());
        assert!(validate_remote_url("not a url").is_err());
    }

    #[test]
    fn offline_uuid_is_stable_and_versioned() {
        let first = offline_uuid("Notch");
        assert_eq!(first, offline_uuid("Notch"));
        assert_ne!(first, offline_uuid("Alex"));
        assert_eq!(first.len(), 36);
        assert_eq!(first.as_bytes()[14], b'3');
        assert!(matches!(first.as_bytes()[19], b'8' | b'9' | b'a' | b'b'));
    }

    #[test]
    fn should_allow_library_handles_os_rules_correctly() {
        use crate::launcher::types::OSCondition;

        let empty_rules: Vec<Rule> = Vec::new();
        assert!(should_allow_library(&empty_rules));

        let osx_only = vec![Rule {
            action: "allow".to_string(),
            os: Some(OSCondition {
                name: Some("osx".to_string()),
                arch: None,
                version: None,
            }),
            features: None,
        }];
        #[cfg(target_os = "linux")]
        assert!(!should_allow_library(&osx_only));

        let allow_all_except_osx = vec![
            Rule {
                action: "allow".to_string(),
                os: None,
                features: None,
            },
            Rule {
                action: "disallow".to_string(),
                os: Some(OSCondition {
                    name: Some("osx".to_string()),
                    arch: None,
                    version: None,
                }),
                features: None,
            },
        ];
        #[cfg(target_os = "linux")]
        assert!(should_allow_library(&allow_all_except_osx));
    }
}
