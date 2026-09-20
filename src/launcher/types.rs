#![allow(dead_code)]
use serde::Deserialize;
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Page {
    Home,
    Instances,
    Settings,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContentKind {
    Mods,
    ResourcePacks,
    Shaders,
}

impl ContentKind {
    pub fn label(self) -> &'static str {
        match self {
            ContentKind::Mods => "Mods",
            ContentKind::ResourcePacks => "Resource Packs",
            ContentKind::Shaders => "Shaders",
        }
    }

    pub fn modrinth_project_type(self) -> &'static str {
        match self {
            ContentKind::Mods => "mod",
            ContentKind::ResourcePacks => "resourcepack",
            ContentKind::Shaders => "shader",
        }
    }

    pub fn install_folder(self) -> &'static str {
        match self {
            ContentKind::Mods => "mods",
            ContentKind::ResourcePacks => "resourcepacks",
            ContentKind::Shaders => "shaderpacks",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum NavLayout {
    TopBar,
    Sidebar,
}

impl Default for NavLayout {
    fn default() -> Self {
        NavLayout::TopBar
    }
}

#[derive(Debug, Clone)]
pub struct MinecraftInstance {
    pub id: usize,
    pub name: String,
    pub version: String,
    pub loader: String,
    pub path: PathBuf,
    pub mod_count: usize,
    pub resource_pack_count: usize,
    pub shader_count: usize,
    pub has_version_files: bool,
    pub has_runnable_jar: bool,
    /// Per-instance memory override (GB). `None` = use the launcher default.
    pub memory_gb: Option<f32>,
    /// Per-instance Java executable override. `None` = use the launcher default.
    pub java_path: Option<String>,
}

#[derive(Debug, Clone)]
pub struct JavaRuntime {
    pub label: String,
    pub path: String,
    pub major: u32,
}

#[derive(Debug, Clone)]
pub struct ContentItem {
    pub id: String,
    pub slug: String,
    pub title: String,
    pub description: String,
    pub project_type: String,
    pub icon_url: Option<String>,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct ModrinthGalleryItem {
    pub url: String,
    #[serde(default)]
    pub raw_url: Option<String>,
    #[serde(default)]
    pub featured: Option<bool>,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ModDetailsTab {
    #[default]
    Description,
    Gallery,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct ModrinthProjectDetails {
    pub id: String,
    pub slug: String,
    pub title: String,
    pub description: String,
    #[serde(default)]
    pub body: Option<String>,
    #[serde(default)]
    pub icon_url: Option<String>,
    #[serde(default)]
    pub categories: Option<Vec<String>>,
    #[serde(default)]
    pub client_side: Option<String>,
    #[serde(default)]
    pub server_side: Option<String>,
    #[serde(default)]
    pub downloads: Option<u64>,
    #[serde(default)]
    pub followers: Option<u64>,
    #[serde(default)]
    pub gallery: Option<Vec<ModrinthGalleryItem>>,
    #[serde(default)]
    pub game_versions: Option<Vec<String>>,
    #[serde(default)]
    pub loaders: Option<Vec<String>>,
    #[serde(default)]
    pub issues_url: Option<String>,
    #[serde(default)]
    pub source_url: Option<String>,
    #[serde(default)]
    pub wiki_url: Option<String>,
    #[serde(default)]
    pub discord_url: Option<String>,
}

#[derive(Debug, Clone)]
pub struct CreateRequest {
    pub root: String,
    pub name: String,
    pub version: String,
    pub loader: String,
    pub player_name: String,
    pub java_path: String,
    pub memory_gb: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Account {
    pub name: String,
    pub uuid: String,
    pub access_token: String,
    pub is_microsoft: bool,
    pub refresh_token: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MicrosoftLoginState {
    pub device_code: String,
    pub user_code: String,
    pub verification_uri: String,
    pub expires_in: u32,
    pub interval: u32,
    pub status: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ModrinthProfile {
    pub path: String,
    pub name: String,
    pub game_version: String,
    pub mod_loader: String,
}

#[derive(Debug, Clone)]
pub enum Message {
    PageChanged(Page),
    SelectInstance(usize),
    DeleteInstance(usize),
    RequestDeleteInstance(usize),
    DeleteInstanceConfirmed(usize),
    CancelDeleteInstance,
    RefreshProfiles,
    PlayPressed,
    OpenCreateDialog,
    CloseCreateDialog,
    CreateNameChanged(String),
    CreateVersionSelected(String),
    CreateLoaderSelected(String),
    ToggleCreateVersionExpanded,
    ToggleSettingsVersionExpanded,
    CreateInstance,
    InstanceCreated(Result<String, String>),
    InstanceSearchChanged(String),
    InstanceSortChanged(InstanceSort),
    InstanceRightClicked(usize),
    CloseContextMenu,
    DuplicateInstance(usize),
    InstanceDuplicated(Result<String, String>),
    OpenInstanceFolder(usize),
    VersionsLoaded(Result<Vec<(String, String)>, String>),
    ShowSnapshotsToggled(bool),
    ReloadVersions,
    JavaScanFinished(Vec<JavaRuntime>),
    JavaSelected(String),
    JavaPathValidated(String, Option<u32>),
    InstallRequiredJava,
    JavaInstalled(Result<String, String>),
    MemoryChanged(f32),
    NewAccountNameChanged(String),
    AddOfflineAccount,
    SelectAccount(usize),
    DeleteAccount(usize),
    GameRootChanged(String),
    JavaPathChanged(String),
    ContentKindSelected(ContentKind),
    ContentQueryChanged(String),
    SearchContent,
    ContentSearchFinished(usize, u64, Result<Vec<ContentItem>, String>),
    SelectContent(String),
    OpenProjectDetails(String),
    ProjectDetailsLoaded(Result<ModrinthProjectDetails, String>),
    CloseProjectDetails,
    ModDetailsTabSelected(ModDetailsTab),
    SelectGalleryImage(usize),
    GalleryImageLoaded(String, Option<iced::widget::image::Handle>),
    OpenModrinthUrl(String),
    InstallSelectedContent,
    ContentInstallProgress(usize, String, String, f32),
    ContentInstalled(usize, String, Result<String, String>),
    InstanceVersionSelected(String),
    InstanceLoaderSelected(String),
    CloseWindow,
    MinimizeWindow,
    MaximizeWindow,
    DragWindow,
    ResizeWindow(iced::window::Direction),
    AnimTick,
    WindowEvent(iced::window::Id, iced::window::Event),
    WindowMaximizedStatusReceived(bool),
    TitleBarPressed,
    WindowPlatformReady,
    EventOccurred(iced::Event),
    AccountRightClicked(usize),
    CloseAccountContextMenu,
    ConfirmDeleteAccount(usize),
    DeleteAccountConfirmed(usize),
    CancelDeleteAccount,
    InstanceDropdownSelected(String),
    SetBrowseContentActive(bool),
    DeleteInstalledFile(String),
    InstalledSearchQueryChanged(String),
    InstallContentDirect(String),
    IconLoaded(String, iced::widget::image::Handle),
    ShowImportDialog(bool),
    ImportModrinthProfiles,
    ModrinthProfilesLoaded(Result<Vec<ModrinthProfile>, String>),
    ImportProfileSelected(ModrinthProfile),
    ProfileImported(Result<String, String>),
    InstallToSystem,
    OpenDataFolder,
    OpenLogsFolder,
    BrowseGameRoot,
    BrowseJavaPath,
    GameRootSelected(Option<PathBuf>),
    JavaPathSelected(Option<PathBuf>),
    StartMicrosoftLogin,
    RefreshMicrosoftAccount,
    MicrosoftDeviceCodeReceived(u64, Result<MicrosoftLoginState, String>),
    MicrosoftLoginFinished(u64, Result<Account, String>),
    MicrosoftAccountRefreshed(String, Result<Account, String>),
    CloseMicrosoftLogin,
    MicrosoftLoginPollTick,
    ExtraJvmArgsChanged(String),
    ExtraGameArgsChanged(String),
    NavLayoutChanged(NavLayout),
    LocalModIconsLoaded(usize, Result<HashMap<String, Vec<u8>>, String>),
    CopyMicrosoftUserCode,
    OpenMicrosoftVerificationUri(String),
    PlayerSkinLoaded(String, Result<(String, Vec<u8>), String>),
    LaunchInstancePrepared(usize, Result<String, String>),
    FileImported(usize, Result<(String, ContentKind), String>),
    SelectImportFile,
    ImportFileSelected(usize, ContentKind, Option<PathBuf>),
    PollWaylandDnd,
    RequestDeleteInstalledFile(String),
    CancelDeleteInstalledFile,
    DeleteInstalledFileConfirmed(String),
    ToggleModEnabled(String, bool),
    CheckModUpdates,
    ModUpdatesChecked(
        usize,
        Result<std::collections::HashMap<String, ModrinthUpdateInfo>, String>,
    ),
    UpdateMod(String),
    UpdateAllMods,
    ModUpdated(usize, String, Result<(), String>),
    StopInstance(usize),
    PollRunningProcesses,
    OpenCloneInstanceDialog(usize),
    CloseCloneInstanceDialog,
    CloneNameChanged(String),
    CloneVersionSelected(String),
    CloneVersionExpanded(bool),
    CloneLoaderSelected(String),
    StartCloneInstance,
    InstanceCloned(Result<CloneResult, String>),
    DismissToast,
    SetInstanceSettingsActive(bool),
    InstanceMemoryChanged(f32),
    InstanceMemoryReset,
    InstanceJavaSelected(String),
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ModrinthUpdateRequest {
    pub hashes: Vec<String>,
    pub algorithm: String,
    pub loaders: Vec<String>,
    pub game_versions: Vec<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ModrinthFileHashes {
    pub sha1: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ModrinthVersionFile {
    pub url: String,
    pub filename: String,
    pub primary: bool,
    pub hashes: ModrinthFileHashes,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ModrinthVersion {
    pub id: String,
    pub project_id: String,
    pub version_number: String,
    #[serde(default)]
    pub dependencies: Vec<ModrinthDependency>,
    pub files: Vec<ModrinthVersionFile>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Default)]
pub struct ModrinthDependency {
    pub project_id: Option<String>,
    pub version_id: Option<String>,
    pub dependency_type: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ModrinthUpdateInfo {
    pub local_filename: String,
    pub download_url: String,
    pub new_filename: String,
    pub new_version: String,
    pub project_id: String,
    pub new_sha1: Option<String>,
}

#[derive(Debug, Clone)]
pub struct CloneResult {
    pub new_name: String,
    pub installed: Vec<String>,
    pub missing: Vec<String>,
}

// Serde models for Minecraft configuration deserialization
#[derive(Deserialize, Debug)]
pub struct VersionManifest {
    pub versions: Vec<ManifestVersion>,
}

#[derive(Deserialize, Debug)]
pub struct ManifestVersion {
    pub id: String,
    pub url: String,
    #[serde(rename = "type", default)]
    pub version_type: String,
}

#[derive(Deserialize, Debug)]
pub struct VersionInfo {
    pub id: String,
    #[serde(rename = "assetIndex")]
    pub asset_index: AssetIndexInfo,
    pub downloads: DownloadsInfo,
    pub libraries: Vec<LibraryInfo>,
    #[serde(rename = "mainClass")]
    pub main_class: String,
    #[serde(rename = "minecraftArguments")]
    pub minecraft_arguments: Option<String>,
    pub arguments: Option<ArgumentsInfo>,
    #[serde(rename = "javaVersion")]
    pub java_version: Option<JavaVersionInfo>,
    #[serde(rename = "type")]
    pub version_type: String,
}

#[derive(Deserialize, Debug)]
pub struct AssetIndexInfo {
    pub id: String,
    pub url: String,
}

#[derive(Deserialize, Debug)]
pub struct DownloadsInfo {
    pub client: DownloadArtifact,
}

#[derive(Deserialize, Debug)]
pub struct DownloadArtifact {
    pub url: String,
    pub sha1: Option<String>,
    pub size: Option<u64>,
}

#[derive(Deserialize, Clone, Debug)]
pub struct LibraryInfo {
    pub name: String,
    pub downloads: Option<LibraryDownloads>,
    pub rules: Option<Vec<Rule>>,
    pub url: Option<String>,
    pub natives: Option<HashMap<String, String>>,
}

#[derive(Deserialize, Clone, Debug)]
pub struct LibraryDownloads {
    pub artifact: Option<LibraryArtifact>,
    pub classifiers: Option<HashMap<String, LibraryArtifact>>,
}

#[derive(Deserialize, Clone, Debug)]
pub struct LibraryArtifact {
    pub path: Option<String>,
    pub url: String,
    pub sha1: Option<String>,
    pub size: Option<u64>,
}

#[derive(Deserialize, Clone, Debug)]
pub struct Rule {
    pub action: String,
    pub os: Option<OSCondition>,
    pub features: Option<std::collections::HashMap<String, bool>>,
}

#[derive(Deserialize, Clone, Debug)]
pub struct OSCondition {
    pub name: Option<String>,
    pub arch: Option<String>,
    pub version: Option<String>,
}

#[derive(Deserialize, Debug)]
pub struct JavaVersionInfo {
    #[serde(rename = "majorVersion")]
    pub major_version: u32,
}

#[derive(Deserialize, Clone, Debug)]
pub struct ArgumentsInfo {
    pub game: Option<Vec<ArgumentValue>>,
    pub jvm: Option<Vec<ArgumentValue>>,
}

#[derive(Deserialize, Clone, Debug)]
#[serde(untagged)]
pub enum ArgumentValue {
    Simple(String),
    Conditional {
        rules: Vec<Rule>,
        value: ArgumentValueInner,
    },
}

#[derive(Deserialize, Clone, Debug)]
#[serde(untagged)]
pub enum ArgumentValueInner {
    Single(String),
    Multiple(Vec<String>),
}

impl ArgumentValue {
    pub fn to_arguments(&self, args: &mut Vec<String>) {
        match self {
            ArgumentValue::Simple(s) => {
                args.push(s.clone());
            }
            ArgumentValue::Conditional { rules, value } => {
                if crate::launcher::utils::should_allow_library(rules) {
                    match value {
                        ArgumentValueInner::Single(s) => {
                            args.push(s.clone());
                        }
                        ArgumentValueInner::Multiple(list) => {
                            args.extend(list.clone());
                        }
                    }
                }
            }
        }
    }
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct LoaderProfile {
    pub id: String,
    pub inherits_from: Option<String>,
    pub main_class: String,
    pub libraries: Vec<LibraryInfo>,
    pub arguments: Option<ArgumentsInfo>,
}

#[derive(Deserialize, Debug)]
pub struct AssetIndex {
    pub objects: HashMap<String, AssetObject>,
}

#[derive(Deserialize, Debug)]
pub struct AssetObject {
    pub hash: String,
    pub size: Option<u64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstanceSort {
    Name,
    Version,
    Loader,
}

impl InstanceSort {
    pub const ALL: [InstanceSort; 3] = [
        InstanceSort::Name,
        InstanceSort::Version,
        InstanceSort::Loader,
    ];

    pub fn label(self) -> &'static str {
        match self {
            InstanceSort::Name => "Name",
            InstanceSort::Version => "Version",
            InstanceSort::Loader => "Loader",
        }
    }
}

impl std::fmt::Display for InstanceSort {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.label())
    }
}
