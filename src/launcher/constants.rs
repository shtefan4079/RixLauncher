//! User-facing and network defaults shared by the launcher.
//!
//! Keeping these values in one place makes it possible to change the launcher
//! identity, endpoints, and safe defaults without hunting through UI and
//! download code. Secrets are deliberately not part of this module.

pub const APP_NAME: &str = "RixLauncher";
pub const APP_VERSION: &str = env!("CARGO_PKG_VERSION");
#[allow(dead_code)]
pub const APPLICATION_ID: &str = "rix_launcher";
pub const USER_AGENT: &str = concat!("RixLauncher/", env!("CARGO_PKG_VERSION"));
pub const USER_DATA_DIRECTORY: &str = ".rixlauncher";
#[cfg(target_os = "windows")]
pub const INSTALL_DIRECTORY_NAME: &str = APP_NAME;
#[allow(dead_code)]
pub const EXECUTABLE_NAME: &str = "rix_launcher";
#[cfg(target_os = "windows")]
pub const WINDOWS_EXECUTABLE_NAME: &str = "rix_launcher.exe";
#[allow(dead_code)]
pub const DESKTOP_ENTRY_NAME: &str = "rix_launcher.desktop";

pub const DEFAULT_WINDOW_WIDTH: f32 = 1_100.0;
pub const DEFAULT_WINDOW_HEIGHT: f32 = 720.0;
pub const MIN_WINDOW_WIDTH: f32 = 900.0;
pub const MIN_WINDOW_HEIGHT: f32 = 620.0;
pub const DEFAULT_CORNER_RADIUS: f32 = 16.0;
pub const STACKED_LAYOUT_WIDTH: i32 = 1_100;
pub const NARROW_LAYOUT_WIDTH: i32 = 1_050;

pub const DEFAULT_MEMORY_GB: f32 = 4.0;
pub const VANILLA_LOADER: &str = "Vanilla";
pub const FABRIC_LOADER: &str = "Fabric";
pub const QUILT_LOADER: &str = "Quilt";
pub const DEFAULT_LOADER: &str = FABRIC_LOADER;
pub const MIN_MEMORY_GB: f32 = 2.0;
pub const MAX_MEMORY_GB: f32 = 16.0;
pub const DOWNLOAD_CONCURRENCY: usize = 16;
pub const DEFAULT_JAVA_COMMAND: &str = "java";
pub const AUTO_JAVA_CONFIG_VALUE: &str = "auto";
pub const OFFLINE_ACCESS_TOKEN: &str = "null";
pub const EMPTY_AUTH_VALUE: &str = "null";
pub const DEFAULT_PLAYER_NAME: &str = "Player";
pub const DEFAULT_INSTANCE_NAME: &str = "New Instance";
#[cfg(windows)]
pub const WINDOWS_CREATE_NO_WINDOW: u32 = 0x08000000;
pub const DEFAULT_MINECRAFT_VERSION: &str = "1.21";
pub const AUTO_JAVA_LABEL: &str = "Auto (launcher default)";
pub const CONTENT_REGISTRY_FILE: &str = ".rixlauncher-content.json";
pub const DOWNLOAD_TIMEOUT_SECONDS: u64 = 90;
pub const SHORT_REQUEST_TIMEOUT_SECONDS: u64 = 30;
pub const IMPORT_REQUEST_TIMEOUT_SECONDS: u64 = 60;
pub const SKIN_REQUEST_TIMEOUT_SECONDS: u64 = 20;
pub const MAX_GAME_LOG_BYTES: u64 = 10 * 1024 * 1024;
pub const MAX_LAUNCHER_LOG_BYTES: u64 = 5 * 1024 * 1024;
pub const MODRINTH_SEARCH_LIMIT: usize = 20;
pub const CURL_CONNECT_TIMEOUT_SECONDS: u64 = 15;
pub const MAX_INSTANCE_NAME_CHARS: usize = 64;
pub const MAX_PLAYER_NAME_CHARS: usize = 16;
pub const ANIMATION_TICK_MILLIS: u64 = 16;
pub const PROCESS_POLL_MILLIS: u64 = 500;
pub const TOAST_DURATION_MILLIS: u64 = 3_000;
#[allow(dead_code)]
pub const WAYLAND_DND_POLL_MILLIS: u64 = 50;
pub const ACCOUNT_PANEL_COMPACT_HEIGHT: f32 = 250.0;
pub const DOUBLE_CLICK_MILLIS: u64 = 300;
pub const OAUTH_CANCEL_POLL_MILLIS: u64 = 250;
pub const DOWNLOAD_ATTEMPTS: u32 = 3;
pub const DOWNLOAD_RETRY_DELAY_MILLIS: u64 = 400;

pub const MOJANG_VERSION_MANIFEST_URL: &str =
    "https://launchermeta.mojang.com/mc/game/version_manifest.json";
pub const MOJANG_LIBRARY_BASE_URL: &str = "https://libraries.minecraft.net/";
pub const MINECRAFT_ASSET_BASE_URL: &str = "https://resources.download.minecraft.net";
pub const MODRINTH_API_BASE_URL: &str = "https://api.modrinth.com/v2";
pub const FABRIC_META_BASE_URL: &str = "https://meta.fabricmc.net/v2";
pub const QUILT_META_BASE_URL: &str = "https://meta.quiltmc.org/v3";
pub const ADOPTIUM_API_BASE_URL: &str = "https://api.adoptium.net/v3";
pub const MINOTAR_BASE_URL: &str = "https://minotar.net";

pub const MICROSOFT_CLIENT_ID: &str = "00000000402b5328";
pub const MICROSOFT_DEVICE_CODE_URL: &str = "https://login.live.com/oauth20_connect.srf";
pub const MICROSOFT_TOKEN_URL: &str = "https://login.live.com/oauth20_token.srf";
pub const MICROSOFT_XBOX_SCOPE: &str = "service::user.auth.xboxlive.com::MBI_SSL";
pub const XBOX_AUTH_URL: &str = "https://user.auth.xboxlive.com/user/authenticate";
pub const XBOX_XSTS_URL: &str = "https://xsts.auth.xboxlive.com/xsts/authorize";
pub const XBOX_AUTH_METHOD: &str = "RPS";
pub const XBOX_SITE_NAME: &str = "user.auth.xboxlive.com";
pub const XBOX_AUTH_RELYING_PARTY: &str = "http://auth.xboxlive.com";
pub const XBOX_TOKEN_TYPE: &str = "JWT";
pub const XBOX_SANDBOX_ID: &str = "RETAIL";
pub const MINECRAFT_XBOX_RELYING_PARTY: &str = "rp://api.minecraftservices.com/";
pub const MINECRAFT_XBOX_LOGIN_URL: &str =
    "https://api.minecraftservices.com/authentication/login_with_xbox";
pub const MINECRAFT_PROFILE_URL: &str = "https://api.minecraftservices.com/minecraft/profile";
pub const ACCOUNT_CREDENTIAL_SERVICE: &str = "com.rixlauncher.credentials";

pub const SUPPORTED_LOADERS: &[&str] = &[VANILLA_LOADER, FABRIC_LOADER, QUILT_LOADER];

pub fn canonical_loader(loader: &str) -> Option<&'static str> {
    SUPPORTED_LOADERS
        .iter()
        .copied()
        .find(|supported| supported.eq_ignore_ascii_case(loader.trim()))
}
