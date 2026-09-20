use crate::launcher::constants;
use crate::launcher::types::JavaRuntime;
use crate::launcher::utils::{
    curl_download, curl_get, publish_replacement, safe_relative_component, unique_temp_path,
};
use std::fs;
use std::path::Path;
use std::process::Command;

pub fn scan_java_runtimes() -> Vec<JavaRuntime> {
    let java_bin_name = java_bin_name();
    let mut candidates = vec![java_bin_name.to_string()];

    let roots = if cfg!(target_os = "windows") {
        vec![
            "C:\\Program Files\\Java".to_string(),
            "C:\\Program Files\\Eclipse Adoptium".to_string(),
            "C:\\Program Files\\Microsoft".to_string(),
            "C:\\Program Files (x86)\\Java".to_string(),
        ]
    } else if cfg!(target_os = "macos") {
        vec!["/Library/Java/JavaVirtualMachines".to_string()]
    } else {
        vec!["/usr/lib/jvm".to_string(), "/usr/java".to_string()]
    };

    for root in roots {
        collect_java_candidates(Path::new(&root), java_bin_name, 3, &mut candidates);
    }

    // Managed runtimes installed by the launcher itself
    let managed_root = managed_java_root();
    collect_java_candidates(&managed_root, java_bin_name, 2, &mut candidates);

    let mut runtimes = Vec::new();
    for path in candidates {
        if let Some(major) = java_major(&path) {
            let label = format!("Java {major} - {path}");
            if !runtimes
                .iter()
                .any(|runtime: &JavaRuntime| runtime.path == path)
            {
                runtimes.push(JavaRuntime { label, path, major });
            }
        }
    }
    runtimes.sort_by_key(|runtime| runtime.major);
    runtimes
}

pub(crate) fn java_bin_name() -> &'static str {
    if cfg!(target_os = "windows") {
        "java.exe"
    } else {
        "java"
    }
}

fn collect_java_candidates(
    root: &Path,
    java_bin_name: &str,
    depth: u8,
    candidates: &mut Vec<String>,
) {
    if depth == 0 {
        return;
    }
    let Ok(entries) = fs::read_dir(root) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            let direct = path.join("bin").join(java_bin_name);
            if direct.is_file() {
                candidates.push(direct.display().to_string());
            }
            collect_java_candidates(&path, java_bin_name, depth - 1, candidates);
        }
    }
}

pub(crate) fn managed_java_root() -> std::path::PathBuf {
    super::get_rixlauncher_root().join("java")
}

pub fn java_major(path: &str) -> Option<u32> {
    let mut cmd = Command::new(path);
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(constants::WINDOWS_CREATE_NO_WINDOW);
    }
    let output = cmd.arg("-version").output().ok()?;
    if !output.status.success() {
        return None;
    }
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stderr),
        String::from_utf8_lossy(&output.stdout)
    );
    let quote = text.find('"')?;
    let after = &text[quote + 1..];
    let end = after.find('"')?;
    let version = &after[..end];
    if let Some(rest) = version.strip_prefix("1.") {
        rest.split('.').next()?.parse().ok()
    } else {
        version.split('.').next()?.parse().ok()
    }
}

/// Installs (or reuses) a JDK with the given major version.
/// Never downloads when a suitable runtime is already available:
/// 1. An already-managed `jdk-<major>` install is returned as-is.
/// 2. Any scanned system runtime with the same major is reused.
/// 3. Only then is a fresh archive downloaded once and extracted atomically,
///    so failed/partial installs never leave a half-broken JDK behind.
pub fn install_java_runtime(major: u32) -> Result<String, String> {
    let java_bin_name = java_bin_name();

    let install_dir = managed_java_root().join(format!("jdk-{major}"));
    let existing_managed = install_dir.join("bin").join(java_bin_name);
    if existing_managed.is_file() && java_major(&existing_managed.display().to_string()).is_some() {
        return Ok(existing_managed.display().to_string());
    }
    // Remove leftovers of a previously interrupted installation
    if install_dir.exists() {
        let _ = fs::remove_dir_all(&install_dir);
    }

    // Reuse any already-installed runtime with the exact major before downloading
    for runtime in scan_java_runtimes() {
        if runtime.major == major {
            return Ok(runtime.path);
        }
    }

    let root = managed_java_root();
    fs::create_dir_all(&root).map_err(|error| error.to_string())?;

    let is_windows = cfg!(target_os = "windows");
    let arch = if cfg!(target_arch = "aarch64") {
        "aarch64"
    } else {
        "x64"
    };
    let os_name = if is_windows {
        "windows"
    } else if cfg!(target_os = "macos") {
        "mac"
    } else {
        "linux"
    };

    let fallback_url = format!(
        "{}/binary/latest/{major}/ga/{os_name}/{arch}/jdk/hotspot/normal/eclipse",
        constants::ADOPTIUM_API_BASE_URL
    );

    let (url, expected_sha256) =
        adoptium_package(&major.to_string(), os_name, arch).unwrap_or((fallback_url, None));

    // Extract into a temp directory first, then move into place atomically so
    // an interrupted download/extraction never looks like a valid install.
    let staging_dir = root.join(format!(".jdk-{major}-staging"));
    let _ = fs::remove_dir_all(&staging_dir);
    fs::create_dir_all(&staging_dir).map_err(|error| error.to_string())?;

    let result = (|| -> Result<(), String> {
        let archive = root.join(format!("jdk-{major}.archive"));
        curl_download(&url, &archive, &[])?;

        if !archive.is_file() || fs::metadata(&archive).map(|m| m.len()).unwrap_or(0) < 1024 {
            return Err("Downloaded Java archive is empty or truncated".to_string());
        }
        if let Some(expected) = expected_sha256.as_deref() {
            let actual = sha256_file(&archive)?;
            if !actual.eq_ignore_ascii_case(expected) {
                return Err("Downloaded Java archive failed checksum verification".to_string());
            }
        }

        if is_windows {
            extract_zip_strip_component(&archive, &staging_dir)?;
        } else {
            let status = Command::new("tar")
                .arg("-xzf")
                .arg(&archive)
                .arg("-C")
                .arg(&staging_dir)
                .arg("--strip-components=1")
                .status()
                .map_err(|error| format!("Could not extract Java: {error}"))?;
            if !status.success() {
                return Err("Java archive extraction failed".to_string());
            }
        }

        let staged_java = staging_dir.join("bin").join(java_bin_name);
        if !staged_java.is_file() || java_major(&staged_java.display().to_string()).is_none() {
            return Err("Extracted JDK does not contain a working java binary".to_string());
        }

        // Clean up the archive to save disk space
        let _ = fs::remove_file(&archive);
        Ok(())
    })();

    if let Err(e) = result {
        let _ = fs::remove_dir_all(&staging_dir);
        return Err(e);
    }

    fs::rename(&staging_dir, &install_dir)
        .map_err(|e| format!("Could not finalize Java installation: {e}"))?;

    Ok(install_dir
        .join("bin")
        .join(java_bin_name)
        .display()
        .to_string())
}

fn adoptium_package(major: &str, os: &str, architecture: &str) -> Option<(String, Option<String>)> {
    let url = format!(
        "{}/assets/latest/{major}/hotspot?architecture={architecture}&os={os}&image_type=jdk&vendor=eclipse",
        constants::ADOPTIUM_API_BASE_URL
    );
    let json = curl_get(&url, &[]).ok()?;
    let first = serde_json::from_str::<serde_json::Value>(&json)
        .ok()?
        .as_array()?
        .first()?
        .clone();
    let package = first.get("binary")?.get("package")?;
    let link = package.get("link")?.as_str()?.to_string();
    let checksum = package
        .get("checksum")
        .and_then(|value| value.as_str())
        .map(ToString::to_string);
    Some((link, checksum))
}

fn sha256_file(path: &Path) -> Result<String, String> {
    use sha2::{Digest, Sha256};
    let file = fs::File::open(path).map_err(|e| e.to_string())?;
    let mut reader = std::io::BufReader::new(file);
    let mut hasher = Sha256::new();
    std::io::copy(&mut reader, &mut hasher).map_err(|e| e.to_string())?;
    Ok(format!("{:x}", hasher.finalize()))
}

fn extract_zip_strip_component(archive_path: &Path, dest_dir: &Path) -> Result<(), String> {
    let file = fs::File::open(archive_path).map_err(|e| e.to_string())?;
    let mut archive = zip::ZipArchive::new(file).map_err(|e| e.to_string())?;

    for i in 0..archive.len() {
        let mut file = archive.by_index(i).map_err(|e| e.to_string())?;
        let filepath = file.enclosed_name().ok_or("Invalid file path in zip")?;

        // Strip the first directory component
        let mut components = filepath.components();
        components.next();
        let stripped_path = components.as_path();

        if stripped_path.as_os_str().is_empty() {
            continue;
        }

        let outpath = dest_dir.join(stripped_path);

        if file.is_dir() {
            fs::create_dir_all(&outpath).map_err(|e| e.to_string())?;
        } else {
            if let Some(p) = outpath.parent() {
                fs::create_dir_all(p).map_err(|e| e.to_string())?;
            }
            let temporary = unique_temp_path(&outpath, "java");
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
        }
    }
    Ok(())
}

/// Heuristic Java requirement for a Minecraft version. Used only as a
/// fallback when the authoritative `javaVersion.majorVersion` field from the
/// version JSON is unavailable (i.e. before the version files are downloaded).
///
/// Handles both the classic scheme ("1.21.11") and the date-based scheme
/// introduced after 1.21.x ("26.2"), which continue where 1.21 left off and
/// therefore require at least Java 21.
pub fn required_java_major(version: &str) -> u32 {
    let mut parts = version
        .split('.')
        .filter_map(|part| part.parse::<u32>().ok());

    let major = parts.next().unwrap_or(1);

    if major >= 26 {
        // Date-based versions (26.x, 27.x, ...) target Java 25+
        return 25;
    }
    if major > 1 {
        return 21;
    }

    let minor = parts.next().unwrap_or(0);
    let patch = parts.next().unwrap_or(0);

    if minor > 20 || (minor == 20 && patch >= 5) {
        21
    } else if minor >= 18 {
        17
    } else if minor >= 17 {
        16
    } else {
        8
    }
}

/// Best-effort lookup of the required Java major from the locally cached
/// version JSON (`<root>/versions/<version>/<version>.json`), falling back to
/// the heuristic parser. The JSON value comes from Mojang and is always
/// authoritative.
pub fn required_java_major_for_version(versions_dir: &Path, version: &str) -> u32 {
    let Some(version_component) = safe_relative_component(version.trim()) else {
        return required_java_major(version);
    };
    let version = version.trim();
    let json_path = versions_dir
        .join(version_component)
        .join(format!("{version}.json"));
    if let Ok(content) = fs::read_to_string(&json_path) {
        if let Ok(value) = serde_json::from_str::<serde_json::Value>(&content) {
            if let Some(major) = value
                .get("javaVersion")
                .and_then(|jv| jv.get("majorVersion"))
                .and_then(|m| m.as_u64())
            {
                if major > 0 {
                    return major as u32;
                }
            }
        }
    }
    required_java_major(version)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_required_java_major() {
        assert_eq!(required_java_major("1.8.9"), 8);
        assert_eq!(required_java_major("1.12.2"), 8);
        assert_eq!(required_java_major("1.16.5"), 8);
        assert_eq!(required_java_major("1.17"), 16);
        assert_eq!(required_java_major("1.17.1"), 16);
        assert_eq!(required_java_major("1.18"), 17);
        assert_eq!(required_java_major("1.19.4"), 17);
        assert_eq!(required_java_major("1.20.1"), 17);
        assert_eq!(required_java_major("1.20.5"), 21);
        assert_eq!(required_java_major("1.21"), 21);
        assert_eq!(required_java_major("1.21.11"), 21);

        // Date-based (post-1.21) versions require modern Java 25+
        assert_eq!(required_java_major("26.1"), 25);
        assert_eq!(required_java_major("26.2"), 25);
        assert_eq!(required_java_major("27.0"), 25);
    }
}
