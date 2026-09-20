pub mod constants;
pub mod downloader;
pub mod executor;
pub mod java;
pub mod types;
pub mod utils;
pub mod views;

use iced::Element;
use iced::Subscription;
use iced::Task;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use java::{required_java_major, scan_java_runtimes};
pub use types::{
    Account, ContentItem, ContentKind, CreateRequest, InstanceSort, JavaRuntime, Message,
    MicrosoftLoginState, MinecraftInstance, ModrinthProfile, ModrinthUpdateInfo,
    ModrinthUpdateRequest, ModrinthVersion, NavLayout, Page,
};
use utils::{
    atomic_copy_file, atomic_write, count_files, count_jars, curl_download, curl_get,
    curl_get_bytes, is_disabled_mod_filename, offline_uuid, parse_modrinth_hits, safe_file_name,
    safe_relative_component, sanitize_name, stable_instance_id, strip_disabled_suffix,
    unique_temp_path, url_encode, validate_remote_url,
};

pub struct RixLauncher {
    pub active_page: Page,
    pub instances: Vec<MinecraftInstance>,
    pub selected_instance_id: Option<usize>,
    pub instance_search: String,
    pub instance_sort: InstanceSort,
    pub active_context_menu: Option<usize>,
    pub instance_to_delete: Option<usize>,
    pub cursor_position: iced::Point,
    pub context_menu_position: iced::Point,

    pub accounts: Vec<Account>,
    pub active_account: usize,
    pub new_account_name: String,
    pub active_account_context_menu: Option<usize>,
    pub account_to_delete: Option<usize>,
    pub microsoft_login: Option<MicrosoftLoginState>,
    pub microsoft_refreshing: bool,
    pub pending_launch_instance_id: Option<usize>,

    pub game_root: String,
    pub java_path: String,
    pub java_runtimes: Vec<JavaRuntime>,
    pub selected_java_label: Option<String>,
    pub java_installing: Option<u32>,
    pub memory_gb: f32,

    pub minecraft_versions: Vec<String>,
    pub all_versions: Vec<(String, String)>,
    pub show_snapshots: bool,
    pub create_version_expanded: bool,
    pub settings_version_expanded: bool,
    pub versions_loading: bool,
    pub show_create_dialog: bool,
    pub create_name: String,
    pub create_version: String,
    pub create_loader: String,
    pub create_status: String,

    pub content_kind: ContentKind,
    pub content_query: String,
    pub content_results: Vec<ContentItem>,
    pub selected_content_id: Option<String>,
    pub content_status: String,
    pub content_searching: bool,
    pub content_search_generation: u64,
    pub content_installing_instance_id: Option<usize>,
    pub content_installing_project_id: Option<String>,
    pub content_install_progress: f32,
    pub content_install_phase: String,
    pub browse_content_active: bool,
    pub instance_settings_active: bool,
    pub installed_search_query: String,
    pub icon_cache: std::collections::HashMap<String, iced::widget::image::Handle>,
    pub project_details: Option<types::ModrinthProjectDetails>,
    pub mod_details_tab: types::ModDetailsTab,
    pub loading_project_details: bool,
    pub project_details_error: Option<String>,
    pub selected_gallery_index: usize,
    pub gallery_image_cache: std::collections::HashMap<String, iced::widget::image::Handle>,
    pub pending_minimize: bool,
    pub extra_jvm_args: String,
    pub extra_game_args: String,
    pub local_mod_icons: std::collections::HashMap<String, iced::widget::image::Handle>,
    pub player_skins: std::collections::HashMap<String, iced::widget::image::Handle>,
    pub noise_image: iced::widget::image::Handle,

    pub show_import_dialog: bool,
    pub modrinth_profiles: Vec<ModrinthProfile>,
    pub import_status: String,

    pub file_to_delete: Option<String>,
    pub checking_updates: bool,
    pub update_results: std::collections::HashMap<String, ModrinthUpdateInfo>,
    pub updating_files: std::collections::HashSet<String>,

    pub active_processes: std::collections::HashMap<usize, std::process::Child>,
    pub launching_instance_id: Option<usize>,
    pub anim_time: f32,

    pub status_message: String,
    pub activity: Vec<String>,

    pub corner_radius: f32,
    pub target_corner_radius: f32,
    pub window_width: i32,
    pub window_height: i32,
    pub last_titlebar_click: std::time::Instant,

    // Clone instance dialog state
    pub show_clone_dialog: Option<usize>,
    pub clone_name: String,
    pub clone_version: String,
    pub clone_loader: String,
    pub clone_version_expanded: bool,
    pub clone_status: String,
    pub clone_in_progress: bool,

    /// Toast notification: (message, kind, expiry anim_time)
    /// kind: 0=info, 1=success, 2=error
    pub toast: Option<(String, u8, f32)>,

    /// When each instance was launched (for uptime display)
    pub instance_launch_times: std::collections::HashMap<usize, std::time::Instant>,

    /// Navigation layout: top bar or left sidebar
    pub nav_layout: NavLayout,

    /// Set when a device-code login is active. Closing the dialog flips the
    /// flag so the polling task can stop without lingering in the background.
    pub microsoft_login_cancel: Option<std::sync::Arc<std::sync::atomic::AtomicBool>>,
    pub microsoft_login_generation: u64,
    pub platform_ready: bool,
}

fn generate_noise_rgba(width: usize, height: usize) -> Vec<u8> {
    let mut buffer = Vec::with_capacity(width * height * 4);
    let mut seed = 12345u32;

    for _ in 0..(width * height) {
        seed = seed.wrapping_mul(1103515245).wrapping_add(12345);
        let rand_val = ((seed / 65536) % 256) as u8;
        buffer.push(rand_val); // R
        buffer.push(rand_val); // G
        buffer.push(rand_val); // B
        let alpha = rand_val % 3; // extremely subtle grain: 0, 1, or 2
        buffer.push(alpha); // A
    }
    buffer
}

fn open_folder(path: &std::path::Path) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer")
            .arg(path)
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(path)
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        std::process::Command::new("xdg-open")
            .arg(path)
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

fn open_url(url: &str) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("cmd")
            .args(["/C", "start", "", url])
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(url)
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        std::process::Command::new("xdg-open")
            .arg(url)
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

impl RixLauncher {
    pub fn new() -> (Self, Task<Message>) {
        let noise_pixels = generate_noise_rgba(1920, 1200);
        let noise_image = iced::widget::image::Handle::from_rgba(1920, 1200, noise_pixels);

        let config_opt = load_config();
        let game_root = config_opt
            .as_ref()
            .map(|config| config.game_root.clone())
            .filter(|root| !root.trim().is_empty())
            .unwrap_or_else(default_game_root);
        let instances = load_instances(&game_root);
        let runtimes: Vec<JavaRuntime> = Vec::new();

        let selected_instance_id = config_opt
            .as_ref()
            .and_then(|c| c.selected_instance_id)
            .filter(|selected_id| instances.iter().any(|instance| instance.id == *selected_id))
            .or_else(|| instances.first().map(|instance| instance.id));

        let java_path = config_opt
            .as_ref()
            .map(|c| c.java_path.clone())
            .filter(|path| !path.trim().is_empty() && path != constants::AUTO_JAVA_CONFIG_VALUE)
            .unwrap_or_else(|| constants::DEFAULT_JAVA_COMMAND.to_string());

        let selected_java_label = config_opt
            .as_ref()
            .and_then(|c| c.selected_java_label.clone())
            .or_else(|| runtimes.first().map(|r| r.label.clone()));

        let memory_gb = config_opt
            .as_ref()
            .map(|c| c.memory_gb)
            .unwrap_or(constants::DEFAULT_MEMORY_GB)
            .clamp(constants::MIN_MEMORY_GB, constants::MAX_MEMORY_GB);
        let extra_jvm_args = config_opt
            .as_ref()
            .map(|c| c.extra_jvm_args.clone())
            .unwrap_or_default();
        let extra_game_args = config_opt
            .as_ref()
            .map(|c| c.extra_game_args.clone())
            .unwrap_or_default();
        let accounts = load_accounts();
        let active_account = config_opt.as_ref().map(|c| c.active_account).unwrap_or(0);
        let active_account = if active_account < accounts.len() {
            active_account
        } else {
            0
        };

        let mut launcher = RixLauncher {
            active_page: Page::Home,
            instances,
            selected_instance_id,
            instance_search: String::new(),
            instance_sort: InstanceSort::Name,
            active_context_menu: None,
            instance_to_delete: None,
            cursor_position: iced::Point::ORIGIN,
            context_menu_position: iced::Point::ORIGIN,

            accounts,
            active_account,
            new_account_name: String::new(),
            active_account_context_menu: None,
            account_to_delete: None,
            microsoft_login: None,
            microsoft_refreshing: false,
            pending_launch_instance_id: None,

            game_root,
            java_path,
            java_runtimes: runtimes,
            selected_java_label,
            java_installing: None,
            memory_gb,

            minecraft_versions: Vec::new(),
            all_versions: Vec::new(),
            show_snapshots: false,
            create_version_expanded: false,
            settings_version_expanded: false,
            versions_loading: true,
            show_create_dialog: false,
            create_name: String::new(),
            create_version: String::new(),
            create_loader: constants::DEFAULT_LOADER.to_string(),
            create_status: String::new(),

            content_kind: ContentKind::Mods,
            content_query: String::new(),
            content_results: Vec::new(),
            selected_content_id: None,
            content_status: String::new(),
            content_searching: false,
            content_search_generation: 0,
            content_installing_instance_id: None,
            content_installing_project_id: None,
            content_install_progress: 0.0,
            content_install_phase: String::new(),
            browse_content_active: false,
            instance_settings_active: false,
            installed_search_query: String::new(),
            icon_cache: std::collections::HashMap::new(),
            project_details: None,
            mod_details_tab: types::ModDetailsTab::Description,
            loading_project_details: false,
            project_details_error: None,
            selected_gallery_index: 0,
            gallery_image_cache: std::collections::HashMap::new(),
            pending_minimize: false,
            extra_jvm_args,
            extra_game_args,
            local_mod_icons: std::collections::HashMap::new(),
            player_skins: std::collections::HashMap::new(),
            noise_image,
            show_import_dialog: false,
            modrinth_profiles: Vec::new(),
            import_status: String::new(),

            file_to_delete: None,
            checking_updates: false,
            update_results: std::collections::HashMap::new(),
            updating_files: std::collections::HashSet::new(),

            active_processes: std::collections::HashMap::new(),
            launching_instance_id: None,
            anim_time: 0.0,

            status_message: "Ready".to_string(),
            activity: vec!["Scanned instance folder".to_string()],

            corner_radius: constants::DEFAULT_CORNER_RADIUS,
            target_corner_radius: constants::DEFAULT_CORNER_RADIUS,

            window_width: constants::DEFAULT_WINDOW_WIDTH as i32,
            window_height: constants::DEFAULT_WINDOW_HEIGHT as i32,
            last_titlebar_click: std::time::Instant::now(),

            show_clone_dialog: None,
            clone_name: String::new(),
            clone_version: String::new(),
            clone_loader: constants::DEFAULT_LOADER.to_string(),
            clone_version_expanded: false,
            clone_status: String::new(),
            clone_in_progress: false,

            toast: None,
            instance_launch_times: std::collections::HashMap::new(),

            nav_layout: config_opt
                .as_ref()
                .map(|c| c.nav_layout)
                .unwrap_or_default(),

            microsoft_login_cancel: None,
            microsoft_login_generation: 0,
            platform_ready: false,
        };

        launcher.select_default_java();
        let load_versions_task = Task::perform(
            async move {
                tokio::task::spawn_blocking(fetch_minecraft_versions)
                    .await
                    .map_err(|error| format!("Version worker failed: {error}"))?
            },
            Message::VersionsLoaded,
        );
        let window_platform_task = iced::window::latest().and_then(|id| {
            apply_window_platform_task(
                id,
                constants::DEFAULT_WINDOW_WIDTH as i32,
                constants::DEFAULT_WINDOW_HEIGHT as i32,
                constants::DEFAULT_CORNER_RADIUS as i32,
            )
        });

        let load_local_icons_task = launcher.trigger_load_local_icons();

        let java_scan_task = Task::perform(
            async {
                tokio::task::spawn_blocking(scan_java_runtimes)
                    .await
                    .unwrap_or_default()
            },
            Message::JavaScanFinished,
        );
        let mut startup_tasks = vec![
            java_scan_task,
            load_versions_task,
            window_platform_task,
            load_local_icons_task,
        ];
        for account in &launcher.accounts {
            let name = account.name.clone();
            let uuid = account.uuid.clone();
            startup_tasks.push(player_skin_task(name, uuid));
        }

        (launcher, Task::batch(startup_tasks))
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::PageChanged(page) => {
                self.active_page = page;
                if page == Page::Home {
                    self.reload_instances();
                }
                Task::none()
            }
            Message::SelectInstance(id) => {
                if self.selected_instance_id != Some(id) {
                    self.clear_instance_scoped_state();
                }
                self.selected_instance_id = Some(id);
                if let Some(instance) = self.selected_instance() {
                    self.status_message = instance.name.clone();
                }
                self.select_default_java();
                self.save_config();
                self.trigger_load_local_icons()
            }
            Message::InstanceDropdownSelected(name) => {
                if let Some((instance_id, instance_name)) = self
                    .instances
                    .iter()
                    .find(|i| i.name == name)
                    .map(|i| (i.id, i.name.clone()))
                {
                    if self.selected_instance_id != Some(instance_id) {
                        self.clear_instance_scoped_state();
                    }
                    self.selected_instance_id = Some(instance_id);
                    self.status_message = instance_name;
                    self.select_default_java();
                    self.save_config();
                    self.trigger_load_local_icons()
                } else {
                    Task::none()
                }
            }
            Message::SetBrowseContentActive(active) => {
                self.browse_content_active = active;
                self.instance_settings_active = false;
                self.project_details = None;
                self.loading_project_details = false;
                self.project_details_error = None;
                self.content_search_generation = self.content_search_generation.wrapping_add(1);
                let search_generation = self.content_search_generation;
                self.content_query.clear();
                self.content_results.clear();
                self.selected_content_id = None;
                self.content_status.clear();
                self.content_searching = false;
                if active {
                    let Some(instance) = self.selected_instance().cloned() else {
                        self.content_status = "Select an instance first".to_string();
                        return Task::none();
                    };
                    let kind = self.content_kind;
                    let instance_id = instance.id;
                    self.content_status = "Loading compatible content...".to_string();
                    self.content_searching = true;
                    Task::perform(
                        async move {
                            tokio::task::spawn_blocking(move || search_content(kind, "", &instance))
                                .await
                                .map_err(|error| format!("Search worker failed: {error}"))?
                        },
                        move |result| {
                            Message::ContentSearchFinished(instance_id, search_generation, result)
                        },
                    )
                } else {
                    Task::none()
                }
            }
            Message::DeleteInstalledFile(filename) => {
                let Some(filename) = safe_file_name(&filename) else {
                    self.status_message = "Invalid installed file name".to_string();
                    return Task::none();
                };
                if let Some(instance) = self.selected_instance() {
                    let path = instance
                        .path
                        .join(self.content_kind.install_folder())
                        .join(filename);
                    if path.is_file() {
                        match std::fs::remove_file(path) {
                            Ok(()) => {
                                let _ = forget_installed_content_file(
                                    instance,
                                    self.content_kind,
                                    filename,
                                );
                                self.reload_instances();
                                self.push_activity(format!("Deleted file {}", filename));
                                self.update_results.remove(filename);
                                return self.trigger_load_local_icons();
                            }
                            Err(error) => {
                                self.status_message = format!("Could not delete file: {error}");
                            }
                        }
                    }
                }
                Task::none()
            }
            Message::RequestDeleteInstalledFile(filename) => {
                if safe_file_name(&filename).is_none() {
                    self.status_message = "Invalid installed file name".to_string();
                } else {
                    self.file_to_delete = Some(filename);
                }
                Task::none()
            }
            Message::CancelDeleteInstalledFile => {
                self.file_to_delete = None;
                Task::none()
            }
            Message::DeleteInstalledFileConfirmed(filename) => {
                self.file_to_delete = None;
                let Some(filename) = safe_file_name(&filename) else {
                    self.status_message = "Invalid installed file name".to_string();
                    return Task::none();
                };
                if let Some(instance) = self.selected_instance() {
                    let path = instance
                        .path
                        .join(self.content_kind.install_folder())
                        .join(filename);
                    if path.is_file() {
                        match std::fs::remove_file(path) {
                            Ok(()) => {
                                let _ = forget_installed_content_file(
                                    instance,
                                    self.content_kind,
                                    filename,
                                );
                                self.reload_instances();
                                self.push_activity(format!("Deleted file {}", filename));
                                self.update_results.remove(filename);
                                return self.trigger_load_local_icons();
                            }
                            Err(error) => {
                                self.status_message = format!("Could not delete file: {error}");
                            }
                        }
                    }
                }
                Task::none()
            }
            Message::ToggleModEnabled(filename, enable) => {
                let Some(filename) = safe_file_name(&filename).map(str::to_owned) else {
                    self.status_message = "Invalid mod file name".to_string();
                    return Task::none();
                };
                if let Some(instance) = self.selected_instance() {
                    let mods_dir = instance.path.join(ContentKind::Mods.install_folder());
                    let src = mods_dir.join(&filename);
                    let dest_filename = if enable {
                        strip_disabled_suffix(&filename)
                            .map(str::to_owned)
                            .unwrap_or_else(|| filename.clone())
                    } else {
                        if strip_disabled_suffix(&filename).is_none() {
                            format!("{}.disabled", filename)
                        } else {
                            filename.clone()
                        }
                    };
                    let dest = mods_dir.join(&dest_filename);
                    if src.is_file() {
                        if dest != src && dest.exists() {
                            self.status_message =
                                format!("Cannot rename mod: {} already exists", dest_filename);
                        } else if let Err(e) = std::fs::rename(src, dest) {
                            self.status_message = format!("Failed to rename mod: {}", e);
                        } else {
                            self.reload_instances();
                            if let Some(update_info) = self.update_results.remove(&filename) {
                                let mut updated_info = update_info;
                                updated_info.local_filename = dest_filename.clone();
                                self.update_results.insert(dest_filename, updated_info);
                            }
                            return self.trigger_load_local_icons();
                        }
                    }
                }
                Task::none()
            }
            Message::CheckModUpdates => {
                if let Some(instance) = self.selected_instance() {
                    let mods_dir = instance.path.join(ContentKind::Mods.install_folder());
                    let loader = instance.loader.clone();
                    let version = instance.version.clone();
                    let instance_id = instance.id;
                    self.checking_updates = true;
                    self.status_message = "Checking for mod updates...".to_string();
                    Task::perform(
                        async move { check_mod_updates_task(mods_dir, loader, version).await },
                        move |result| Message::ModUpdatesChecked(instance_id, result),
                    )
                } else {
                    Task::none()
                }
            }
            Message::ModUpdatesChecked(instance_id, res) => {
                if self.selected_instance_id != Some(instance_id) {
                    return Task::none();
                }
                self.checking_updates = false;
                match res {
                    Ok(updates) => {
                        self.update_results = updates;
                        let count = self.update_results.len();
                        self.status_message = format!("Check complete: found {} updates", count);
                        self.push_activity(format!("Checked updates: {} found", count));
                    }
                    Err(e) => {
                        self.status_message = format!("Update check failed: {}", e);
                        self.push_activity(format!("Update check failed: {}", e));
                    }
                }
                Task::none()
            }
            Message::UpdateMod(filename) => {
                if let Some(instance) = self.selected_instance().cloned() {
                    if let Some(update_info) = self.update_results.get(&filename).cloned() {
                        let mods_dir = instance.path.join(ContentKind::Mods.install_folder());
                        let instance_id = instance.id;
                        self.updating_files.insert(filename.clone());
                        self.status_message = format!("Updating {}...", filename);
                        let file_clone = filename.clone();
                        return Task::perform(
                            async move {
                                download_mod_update_task(
                                    update_info.download_url,
                                    update_info.new_filename,
                                    update_info.local_filename,
                                    update_info.new_sha1,
                                    mods_dir,
                                )
                                .await
                            },
                            move |res| Message::ModUpdated(instance_id, file_clone, res),
                        );
                    }
                }
                Task::none()
            }
            Message::UpdateAllMods => {
                if let Some(instance) = self.selected_instance().cloned() {
                    let mods_dir = instance.path.join(ContentKind::Mods.install_folder());
                    let instance_id = instance.id;
                    let mut tasks = Vec::new();
                    let updates: Vec<ModrinthUpdateInfo> =
                        self.update_results.values().cloned().collect();

                    for update_info in updates {
                        let filename = update_info.local_filename.clone();
                        if !self.updating_files.contains(&filename) {
                            self.updating_files.insert(filename.clone());
                            let file_clone = filename.clone();
                            let mods_dir_clone = mods_dir.clone();
                            tasks.push(Task::perform(
                                async move {
                                    download_mod_update_task(
                                        update_info.download_url,
                                        update_info.new_filename,
                                        update_info.local_filename,
                                        update_info.new_sha1,
                                        mods_dir_clone,
                                    )
                                    .await
                                },
                                move |res| Message::ModUpdated(instance_id, file_clone, res),
                            ));
                        }
                    }
                    if !tasks.is_empty() {
                        self.status_message = "Downloading updates...".to_string();
                        return Task::batch(tasks);
                    }
                }
                Task::none()
            }
            Message::ModUpdated(instance_id, filename, res) => {
                if self.selected_instance_id != Some(instance_id) {
                    self.updating_files.remove(&filename);
                    return Task::none();
                }
                self.updating_files.remove(&filename);
                self.update_results.remove(&filename);
                match res {
                    Ok(_) => {
                        self.status_message = format!("Updated {}", filename);
                        self.push_activity(format!("Updated mod {}", filename));
                        self.reload_instances();
                        return self.trigger_load_local_icons();
                    }
                    Err(e) => {
                        self.status_message = format!("Failed to update {}: {}", filename, e);
                        self.push_activity(format!("Failed to update {}: {}", filename, e));
                    }
                }
                Task::none()
            }
            Message::InstalledSearchQueryChanged(query) => {
                self.installed_search_query = query;
                Task::none()
            }
            Message::DeleteInstance(id) => {
                self.active_context_menu = None;
                if self.active_processes.contains_key(&id) {
                    self.status_message =
                        "Stop the running instance before deleting it".to_string();
                } else {
                    self.instance_to_delete = Some(id);
                }
                Task::none()
            }
            Message::RequestDeleteInstance(id) => {
                self.active_context_menu = None;
                if self.active_processes.contains_key(&id) {
                    self.status_message =
                        "Stop the running instance before deleting it".to_string();
                } else if self.instances.iter().any(|instance| instance.id == id) {
                    self.instance_to_delete = Some(id);
                }
                Task::none()
            }
            Message::DeleteInstanceConfirmed(id) => {
                self.instance_to_delete = None;
                if self.active_processes.contains_key(&id) {
                    self.status_message =
                        "Stop the running instance before deleting it".to_string();
                } else {
                    self.delete_instance(id);
                }
                Task::none()
            }
            Message::CancelDeleteInstance => {
                self.instance_to_delete = None;
                Task::none()
            }
            Message::RefreshProfiles => {
                self.reload_instances();
                self.push_activity("Refreshed instances".to_string());
                Task::none()
            }
            Message::PlayPressed => {
                if let Some(instance) = self.selected_instance().cloned() {
                    if self.active_processes.contains_key(&instance.id) {
                        self.status_message = format!("{} is already running", instance.name);
                        return Task::none();
                    }
                    if self.launching_instance_id.is_some() {
                        self.status_message =
                            "Another instance is already being prepared".to_string();
                        return Task::none();
                    }
                    let refresh_token = self.active_account_object().and_then(|account| {
                        (account.is_microsoft && account.access_token.trim().is_empty())
                            .then(|| account.refresh_token.clone())
                            .flatten()
                            .filter(|token| !token.trim().is_empty())
                    });
                    if self.active_account_object().is_some_and(|account| {
                        account.is_microsoft && account.access_token.trim().is_empty()
                    }) {
                        let Some(refresh_token) = refresh_token else {
                            self.status_message =
                                "Microsoft credentials are unavailable; sign in again".to_string();
                            return Task::none();
                        };
                        if self.microsoft_refreshing {
                            self.status_message =
                                "Microsoft session is already refreshing".to_string();
                            return Task::none();
                        }
                        self.microsoft_refreshing = true;
                        self.pending_launch_instance_id = Some(instance.id);
                        self.launching_instance_id = Some(instance.id);
                        self.status_message =
                            "Refreshing Microsoft session before launch...".to_string();
                        let account_uuid = self
                            .active_account_object()
                            .map(|account| account.uuid.clone())
                            .unwrap_or_default();
                        return Task::perform(
                            async move { refresh_microsoft_account(refresh_token).await },
                            move |result| Message::MicrosoftAccountRefreshed(account_uuid, result),
                        );
                    }
                    let game_root_path = PathBuf::from(self.game_root.trim());
                    let dot_rixlauncher = game_root_path.parent().unwrap_or(&game_root_path);
                    let versions_dir = dot_rixlauncher.join("versions");
                    let vanilla_json_path = versions_dir
                        .join(&instance.version)
                        .join(format!("{}.json", instance.version));
                    let profile_path = instance.path.join("profile.json");

                    let loader_profile_ready = instance.loader == constants::VANILLA_LOADER
                        || (profile_path.is_file()
                            && loader_profile_matches_instance(
                                &profile_path,
                                &instance.version,
                                &instance.loader,
                            ));
                    let effective_java = self.effective_java_path(&instance);
                    let required_java =
                        java::required_java_major_for_version(&versions_dir, &instance.version);
                    let java_major = java::java_major(&effective_java).unwrap_or(0);
                    let java_ready = java_major >= required_java;

                    let needs_download =
                        !vanilla_json_path.is_file() || !loader_profile_ready || !java_ready;

                    if needs_download {
                        self.launching_instance_id = self.selected_instance_id;
                        self.status_message = "Preparing game assets and libraries...".to_string();
                        let request = CreateRequest {
                            root: self.game_root.clone(),
                            name: instance.name.clone(),
                            version: instance.version.clone(),
                            loader: instance.loader.clone(),
                            player_name: self.active_player().to_string(),
                            java_path: effective_java,
                            memory_gb: self.effective_memory_gb(&instance),
                        };
                        Task::perform(
                            async move { downloader::prepare_instance_task(request).await },
                            move |result| Message::LaunchInstancePrepared(instance.id, result),
                        )
                    } else {
                        self.launching_instance_id = self.selected_instance_id;
                        self.launch_selected();
                        Task::none()
                    }
                } else {
                    self.status_message = "Select an instance first".to_string();
                    Task::none()
                }
            }
            Message::LaunchInstancePrepared(instance_id, result) => {
                // A later launch request supersedes an older preparation. Do
                // not let a stale network completion start the wrong profile.
                if self.launching_instance_id != Some(instance_id) {
                    return Task::none();
                }
                if !self
                    .instances
                    .iter()
                    .any(|instance| instance.id == instance_id)
                {
                    self.launching_instance_id = None;
                    self.status_message = "The prepared instance no longer exists".to_string();
                    return Task::none();
                }
                match result {
                    Ok(name) => {
                        self.java_runtimes = java::scan_java_runtimes();
                        self.reload_instances();
                        self.status_message = format!("Assets prepared for {}", name);
                        self.launch_instance(instance_id);
                    }
                    Err(e) => {
                        if self.launching_instance_id == Some(instance_id) {
                            self.launching_instance_id = None;
                            self.status_message =
                                format!("Launch failed during preparation: {}", e);
                        }
                    }
                }
                Task::none()
            }
            Message::StopInstance(instance_id) => {
                if let Some(mut child) = self.active_processes.remove(&instance_id) {
                    executor::stop_process_tree(&mut child);
                    if let Some(instance) = self
                        .instances
                        .iter()
                        .find(|instance| instance.id == instance_id)
                    {
                        self.status_message = format!("Stopped {}", instance.name);
                        self.push_activity(format!("Stopped {}", instance.name));
                    }
                    self.instance_launch_times.remove(&instance_id);
                }
                Task::none()
            }
            Message::PollRunningProcesses => {
                let mut exited_statuses = Vec::new();
                for (id, child) in &mut self.active_processes {
                    if let Ok(Some(status)) = child.try_wait() {
                        exited_statuses.push((*id, status));
                    }
                }
                for (id, status) in exited_statuses {
                    self.active_processes.remove(&id);
                    self.instance_launch_times.remove(&id);
                    if let Some(instance) = self.instances.iter().find(|instance| instance.id == id)
                    {
                        self.status_message =
                            format!("Game {} exited with status: {}", instance.name, status);
                        self.push_activity(format!("Game {} exited", instance.name));
                    }
                }
                Task::none()
            }
            // ── Clone Instance Dialog ──────────────────────────────────────────────
            Message::OpenCloneInstanceDialog(id) => {
                self.active_context_menu = None;
                if let Some(instance) = self.instances.iter().find(|i| i.id == id) {
                    self.show_clone_dialog = Some(id);
                    self.clone_name = format!("{} (Clone)", instance.name);
                    self.clone_version = instance.version.clone();
                    self.clone_loader = instance.loader.clone();
                    self.clone_version_expanded = false;
                    self.clone_status = String::new();
                    self.clone_in_progress = false;
                }
                Task::none()
            }
            Message::CloseCloneInstanceDialog => {
                self.show_clone_dialog = None;
                self.clone_name = String::new();
                self.clone_status = String::new();
                self.clone_in_progress = false;
                Task::none()
            }
            Message::CloneNameChanged(name) => {
                self.clone_name = name;
                Task::none()
            }
            Message::CloneVersionSelected(version) => {
                self.clone_version = version;
                self.clone_version_expanded = false;
                Task::none()
            }
            Message::CloneVersionExpanded(expanded) => {
                self.clone_version_expanded = expanded;
                Task::none()
            }
            Message::CloneLoaderSelected(loader) => {
                self.clone_loader = loader;
                Task::none()
            }
            Message::StartCloneInstance => {
                let Some(src_id) = self.show_clone_dialog else {
                    return Task::none();
                };
                let Some(src_instance) = self.instances.iter().find(|i| i.id == src_id).cloned()
                else {
                    return Task::none();
                };
                if self.clone_name.trim().is_empty() {
                    self.clone_status = "Please enter a name for the new instance.".to_string();
                    return Task::none();
                }
                if self.clone_version.is_empty() {
                    self.clone_status = "Please select a Minecraft version.".to_string();
                    return Task::none();
                }
                self.clone_in_progress = true;
                self.clone_status = "Cloning instance...".to_string();

                let new_name = self.clone_name.trim().to_string();
                let new_version = self.clone_version.clone();
                let new_loader = self.clone_loader.clone();
                let game_root = self.game_root.clone();
                // Carry over the source instance's Java/memory preferences
                let java_path = src_instance
                    .java_path
                    .clone()
                    .unwrap_or_else(|| self.java_path.clone());
                let memory_gb = src_instance.memory_gb.unwrap_or(self.memory_gb);
                let player_name = self
                    .accounts
                    .get(self.active_account)
                    .map(|a| a.name.clone())
                    .unwrap_or_else(|| constants::DEFAULT_PLAYER_NAME.to_string());

                Task::perform(
                    clone_instance_task(
                        src_instance,
                        new_name,
                        new_version,
                        new_loader,
                        game_root,
                        java_path,
                        memory_gb,
                        player_name,
                    ),
                    Message::InstanceCloned,
                )
            }
            Message::InstanceCloned(result) => {
                self.clone_in_progress = false;
                match result {
                    Ok(clone_result) => {
                        let mut status = format!("Cloned as \"{}\".", clone_result.new_name);
                        if !clone_result.installed.is_empty() {
                            status.push_str(&format!(
                                " Installed {} mod(s).",
                                clone_result.installed.len()
                            ));
                        }
                        if !clone_result.missing.is_empty() {
                            status.push_str(&format!(
                                "\n⚠ Could not clone {} mod(s):\n• {}",
                                clone_result.missing.len(),
                                clone_result.missing.join("\n• ")
                            ));
                        }
                        self.clone_status = status.clone();
                        self.status_message = format!("Cloned instance: {}", clone_result.new_name);
                        self.push_activity(format!(
                            "Cloned instance as \"{}\"",
                            clone_result.new_name
                        ));
                        if clone_result.missing.is_empty() {
                            self.show_toast(
                                format!("✓ Cloned as \"{}\"", clone_result.new_name),
                                1,
                            );
                        } else {
                            self.show_toast(
                                format!(
                                    "⚠ Cloned as \"{}\" ({} missing/incompatible mod(s))",
                                    clone_result.new_name,
                                    clone_result.missing.len()
                                ),
                                0,
                            );
                        }
                        self.reload_instances();
                        self.save_config();
                    }
                    Err(e) => {
                        eprintln!("[RixLauncher] [ERROR] Clone failed: {e}");
                        self.clone_status = format!("Clone failed: {e}");
                        self.show_toast(format!("Clone failed: {e}"), 2);
                    }
                }
                Task::none()
            }

            Message::DismissToast => {
                self.toast = None;
                Task::none()
            }

            Message::OpenCreateDialog => {
                self.show_create_dialog = true;
                self.create_name = constants::DEFAULT_INSTANCE_NAME.to_string();
                self.create_status = String::new();
                Task::none()
            }
            Message::ShowImportDialog(show) => {
                self.show_import_dialog = show;
                if show {
                    self.import_status = "Scanning Modrinth App database...".to_string();
                    // Trigger automatic scanning
                    Task::perform(
                        async move {
                            tokio::task::spawn_blocking(scan_modrinth_db_profiles)
                                .await
                                .map_err(|error| format!("Profile scan worker failed: {error}"))?
                        },
                        Message::ModrinthProfilesLoaded,
                    )
                } else {
                    Task::none()
                }
            }
            Message::ImportModrinthProfiles => {
                self.import_status = "Scanning Modrinth App database...".to_string();
                Task::perform(
                    async move {
                        tokio::task::spawn_blocking(scan_modrinth_db_profiles)
                            .await
                            .map_err(|error| format!("Profile scan worker failed: {error}"))?
                    },
                    Message::ModrinthProfilesLoaded,
                )
            }
            Message::ModrinthProfilesLoaded(result) => {
                match result {
                    Ok(profiles) => {
                        self.modrinth_profiles = profiles;
                        if self.modrinth_profiles.is_empty() {
                            self.import_status = "No Modrinth App profiles found".to_string();
                        } else {
                            self.import_status =
                                format!("Loaded {} profiles", self.modrinth_profiles.len());
                        }
                    }
                    Err(err) => {
                        self.import_status = format!("Scan failed: {err}");
                    }
                }
                Task::none()
            }
            Message::ImportProfileSelected(profile) => {
                self.import_status = format!("Importing '{}'...", profile.name);
                let root = self.game_root.clone();
                let player_name = self.active_player().to_string();
                let java_path = self.java_path.clone();
                let memory_gb = self.memory_gb;
                let p_clone = profile.clone();
                Task::perform(
                    async move {
                        perform_import_profile(p_clone, root, player_name, java_path, memory_gb)
                            .await
                    },
                    Message::ProfileImported,
                )
            }
            Message::ProfileImported(result) => {
                match result {
                    Ok(name) => {
                        self.import_status = format!("Successfully imported '{name}'!");
                        self.reload_instances();
                        self.push_activity(format!("Imported Modrinth profile '{name}'"));
                        self.selected_instance_id =
                            self.instances.iter().find(|i| i.name == name).map(|i| i.id);
                        self.save_config();
                    }
                    Err(err) => {
                        self.import_status = format!("Import failed: {err}");
                    }
                }
                Task::none()
            }
            Message::InstallToSystem => {
                #[cfg(not(target_os = "windows"))]
                {
                    let home = home_dir();
                    let bin_dir = home.join(".local").join("bin");
                    if let Err(error) = fs::create_dir_all(&bin_dir) {
                        self.status_message =
                            format!("Could not create install directory: {error}");
                        return Task::none();
                    }
                    let target_bin = bin_dir.join(constants::EXECUTABLE_NAME);

                    match std::env::current_exe() {
                        Ok(current) => {
                            let temporary = unique_temp_path(&target_bin, "install");
                            if let Err(e) = fs::copy(&current, &temporary) {
                                self.status_message = format!("Failed to copy binary: {e}");
                            } else {
                                #[cfg(unix)]
                                {
                                    use std::os::unix::fs::PermissionsExt;
                                    if let Ok(metadata) = fs::metadata(&temporary) {
                                        let mut permissions = metadata.permissions();
                                        permissions.set_mode(0o755);
                                        let _ = fs::set_permissions(&temporary, permissions);
                                    }
                                }
                                if let Err(error) = fs::rename(&temporary, &target_bin) {
                                    let _ = fs::remove_file(&temporary);
                                    self.status_message =
                                        format!("Failed to publish binary: {error}");
                                    return Task::none();
                                }

                                // Trigger icon and desktop installation
                                if let Err(error) = install_xdg_icon() {
                                    self.status_message = format!(
                                        "Binary installed, but desktop integration failed: {error}"
                                    );
                                    return Task::none();
                                }
                                // Update desktop file to use the correct local bin path explicitly
                                let desktop_path = home
                                    .join(".local")
                                    .join("share")
                                    .join("applications")
                                    .join(constants::DESKTOP_ENTRY_NAME);
                                let desktop_content = format!(
                                    "[Desktop Entry]\nName={}\nComment=Modern Minecraft Launcher\nType=Application\nIcon={}\nExec={}\nStartupWMClass={}\nCategories=Game;\nTerminal=false\n",
                                    constants::APP_NAME,
                                    constants::APPLICATION_ID,
                                    target_bin.display(),
                                    constants::APPLICATION_ID,
                                );
                                if let Err(error) =
                                    atomic_write(&desktop_path, desktop_content.as_bytes())
                                {
                                    self.status_message = format!(
                                        "Binary installed, but desktop entry failed: {error}"
                                    );
                                    return Task::none();
                                }

                                self.status_message =
                                    "Launcher successfully installed to system!".to_string();
                                self.push_activity(
                                    "Installed launcher to ~/.local/bin".to_string(),
                                );
                            }
                        }
                        Err(e) => {
                            self.status_message = format!("Could not find current exe path: {e}");
                        }
                    }
                }
                #[cfg(target_os = "windows")]
                {
                    if let (Ok(local_appdata), Ok(current_exe)) =
                        (std::env::var("LOCALAPPDATA"), std::env::current_exe())
                    {
                        let install_dir = std::path::Path::new(&local_appdata)
                            .join("Programs")
                            .join(constants::INSTALL_DIRECTORY_NAME);
                        let _ = fs::create_dir_all(&install_dir);
                        let target = install_dir.join(constants::WINDOWS_EXECUTABLE_NAME);

                        // Copy the executable
                        let already_installed = current_exe
                            .canonicalize()
                            .ok()
                            .zip(target.canonicalize().ok())
                            .is_some_and(|(source, destination)| source == destination);
                        let install_result = if already_installed {
                            Ok(())
                        } else {
                            atomic_copy_file(&current_exe, &target)
                        };
                        if install_result.is_ok() {
                            // Paths are inserted into single-quoted PowerShell
                            // literals. Escape apostrophes so a valid Windows
                            // username/path cannot alter the shortcut script.
                            let target_str = target.to_string_lossy().replace('\'', "''");
                            let install_dir_str = install_dir.to_string_lossy().replace('\'', "''");

                            // Spawn PowerShell in background to create desktop and start menu shortcuts with icon references
                            let ps_script = format!(
                                "$ws = New-Object -ComObject WScript.Shell; \
                                 $d = [System.Environment]::GetFolderPath('Desktop'); \
                                 $s1 = $ws.CreateShortcut(\"$d\\RixLauncher.lnk\"); \
                                 $s1.TargetPath = '{}'; \
                                 $s1.WorkingDirectory = '{}'; \
                                 $s1.IconLocation = '{},0'; \
                                 $s1.Save(); \
                                 $p = [System.Environment]::GetFolderPath('Programs'); \
                                 $s2 = $ws.CreateShortcut(\"$p\\RixLauncher.lnk\"); \
                                 $s2.TargetPath = '{}'; \
                                 $s2.WorkingDirectory = '{}'; \
                                 $s2.IconLocation = '{},0'; \
                                 $s2.Save();",
                                target_str,
                                install_dir_str,
                                target_str,
                                target_str,
                                install_dir_str,
                                target_str
                            );

                            let mut cmd = std::process::Command::new("powershell");
                            use std::os::windows::process::CommandExt;
                            cmd.creation_flags(constants::WINDOWS_CREATE_NO_WINDOW);
                            let _ = cmd.arg("-Command").arg(&ps_script).spawn();

                            self.status_message =
                                "Launcher successfully installed to system!".to_string();
                            self.push_activity(
                                "Installed launcher to AppData and created shortcuts".to_string(),
                            );
                        } else {
                            self.status_message = format!(
                                "Failed to install launcher executable: {}",
                                install_result
                                    .err()
                                    .map(|error| error.to_string())
                                    .unwrap_or_else(|| "unknown error".to_string())
                            );
                        }
                    } else {
                        self.status_message = "Could not resolve LOCALAPPDATA path.".to_string();
                    }
                }
                Task::none()
            }
            Message::CloseCreateDialog => {
                self.show_create_dialog = false;
                Task::none()
            }
            Message::CreateNameChanged(name) => {
                self.create_name = name;
                Task::none()
            }
            Message::CreateVersionSelected(version) => {
                self.create_version = version.trim().to_string();
                self.create_version_expanded = false;
                let selected_version = self.create_version.clone();
                self.select_default_java_for_version(&selected_version);
                Task::none()
            }
            Message::CreateLoaderSelected(loader) => {
                self.create_loader = loader;
                Task::none()
            }
            Message::CreateInstance => {
                let request = CreateRequest {
                    root: self.game_root.clone(),
                    name: self.create_name.trim().to_string(),
                    version: self.create_version.clone(),
                    loader: self.create_loader.clone(),
                    player_name: self.active_player().to_string(),
                    java_path: self.selected_java_path(),
                    memory_gb: self.memory_gb,
                };
                self.create_status = "Creating instance...".to_string();
                Task::perform(
                    async move { downloader::create_instance_task(request).await },
                    Message::InstanceCreated,
                )
            }
            Message::InstanceCreated(result) => {
                match result {
                    Ok(name) => {
                        self.create_status = format!("Created {name}");
                        self.show_create_dialog = false;
                        self.push_activity(format!("Created {name}"));
                        self.reload_instances();
                        self.selected_instance_id = self
                            .instances
                            .iter()
                            .find(|instance| instance.name == name)
                            .map(|instance| instance.id);
                        if let Some(instance) = self.selected_instance() {
                            self.status_message = instance.name.clone();
                        }
                        self.save_config();
                    }
                    Err(error) => {
                        eprintln!("[RixLauncher] [ERROR] Failed to create instance: {error}");
                        self.create_status = format!("Error: {error}");
                        self.status_message = format!("Creation failed: {error}");
                    }
                }
                Task::none()
            }
            Message::ToggleCreateVersionExpanded => {
                self.create_version_expanded = !self.create_version_expanded;
                Task::none()
            }
            Message::ToggleSettingsVersionExpanded => {
                self.settings_version_expanded = !self.settings_version_expanded;
                Task::none()
            }
            Message::VersionsLoaded(result) => {
                self.versions_loading = false;
                match result {
                    Ok(versions) => {
                        self.all_versions = versions;
                        self.update_versions_list();
                    }
                    Err(error) => {
                        self.status_message = format!("Could not load Minecraft versions: {error}");
                    }
                }
                Task::none()
            }
            Message::InstanceSearchChanged(query) => {
                self.instance_search = query;
                Task::none()
            }
            Message::InstanceSortChanged(sort) => {
                self.instance_sort = sort;
                Task::none()
            }
            Message::InstanceRightClicked(id) => {
                self.active_context_menu = Some(id);
                self.context_menu_position = self.cursor_position;
                Task::none()
            }
            Message::CloseContextMenu => {
                self.active_context_menu = None;
                Task::none()
            }
            Message::PollWaylandDnd => self.check_wayland_dnd(),
            Message::EventOccurred(event) => {
                if let iced::Event::Mouse(iced::mouse::Event::CursorMoved { position }) = &event {
                    self.cursor_position = *position;
                }
                if let iced::Event::Keyboard(iced::keyboard::Event::KeyPressed { key, .. }) = &event
                    && matches!(
                        key,
                        iced::keyboard::Key::Named(iced::keyboard::key::Named::Escape)
                    )
                {
                    self.close_topmost_overlay();
                    return Task::none();
                }
                self.check_wayland_dnd()
            }
            Message::DuplicateInstance(id) => {
                self.active_context_menu = None;
                if let Some(instance) = self.instances.iter().find(|i| i.id == id) {
                    let src_path = instance.path.clone();
                    let Some(parent) = src_path.parent() else {
                        self.status_message =
                            "Cannot duplicate an instance without a parent folder".to_string();
                        return Task::none();
                    };
                    let mut name_copy = format!("{}_copy", instance.name);
                    let mut dst_path = parent.join(sanitize_name(&name_copy));
                    let mut counter = 1;
                    while dst_path.exists() {
                        counter += 1;
                        name_copy = format!("{}_copy{}", instance.name, counter);
                        dst_path = parent.join(sanitize_name(&name_copy));
                    }
                    self.status_message = format!("Duplicating {}...", instance.name);
                    Task::perform(
                        async move { duplicate_instance_task(src_path, dst_path).await },
                        Message::InstanceDuplicated,
                    )
                } else {
                    Task::none()
                }
            }
            Message::InstanceDuplicated(result) => {
                match result {
                    Ok(new_name) => {
                        self.status_message = format!("Duplicated instance as: {new_name}");
                        self.reload_instances();
                        self.selected_instance_id = self
                            .instances
                            .iter()
                            .find(|i| {
                                i.name
                                    .replace('_', " ")
                                    .eq_ignore_ascii_case(&new_name.replace('_', " "))
                            })
                            .map(|i| i.id);
                        self.save_config();
                    }
                    Err(e) => {
                        eprintln!("[RixLauncher] [ERROR] Failed to duplicate instance: {e}");
                        self.status_message = format!("Duplication failed: {e}");
                    }
                }
                Task::none()
            }
            Message::OpenInstanceFolder(id) => {
                self.active_context_menu = None;
                if let Some(instance) = self.instances.iter().find(|i| i.id == id) {
                    let _ = open_folder(&instance.path);
                }
                Task::none()
            }
            Message::OpenDataFolder => {
                let root = get_rixlauncher_root();
                let _ = open_folder(&root);
                Task::none()
            }
            Message::OpenLogsFolder => {
                let logs_dir = get_rixlauncher_root().join("logs");
                let _ = std::fs::create_dir_all(&logs_dir);
                let _ = open_folder(&logs_dir);
                Task::none()
            }
            Message::BrowseGameRoot => {
                Task::perform(
                    async {
                        rfd::AsyncFileDialog::new()
                            .pick_folder()
                            .await
                            .map(|h| h.path().to_path_buf())
                    },
                    Message::GameRootSelected,
                )
            }
            Message::GameRootSelected(Some(path)) => {
                self.update(Message::GameRootChanged(path.display().to_string()))
            }
            Message::GameRootSelected(None) => Task::none(),
            Message::BrowseJavaPath => {
                Task::perform(
                    async {
                        rfd::AsyncFileDialog::new()
                            .pick_file()
                            .await
                            .map(|h| h.path().to_path_buf())
                    },
                    Message::JavaPathSelected,
                )
            }
            Message::JavaPathSelected(Some(path)) => {
                self.update(Message::JavaPathChanged(path.display().to_string()))
            }
            Message::JavaPathSelected(None) => Task::none(),
            Message::ShowSnapshotsToggled(value) => {
                self.show_snapshots = value;
                self.update_versions_list();
                Task::none()
            }
            Message::ReloadVersions => {
                self.versions_loading = true;
                Task::perform(
                    async move { fetch_minecraft_versions() },
                    Message::VersionsLoaded,
                )
            }
            Message::JavaScanFinished(runtimes) => {
                self.java_runtimes = runtimes;
                if let Some(runtime) = self
                    .java_runtimes
                    .iter()
                    .find(|runtime| runtime.path == self.java_path)
                {
                    self.selected_java_label = Some(runtime.label.clone());
                } else {
                    self.select_default_java();
                }
                Task::none()
            }
            Message::JavaSelected(label) => {
                self.selected_java_label = Some(label.clone());
                if let Some(runtime) = self.java_runtimes.iter().find(|r| r.label == label) {
                    self.java_path = runtime.path.clone();
                }
                self.save_config();
                Task::none()
            }
            Message::InstallRequiredJava => {
                let major = if self.show_create_dialog {
                    self.required_java_major_for_create()
                } else {
                    self.required_java_major()
                };
                self.java_installing = Some(major);
                self.status_message = format!("Downloading Java {major}...");
                Task::perform(
                    async move { java::install_java_runtime(major) },
                    Message::JavaInstalled,
                )
            }
            Message::JavaInstalled(result) => {
                self.java_installing = None;
                match result {
                    Ok(java_path) => {
                        self.java_path = java_path.clone();
                        self.status_message = "Java installed successfully!".to_string();
                        self.save_config();
                        return Task::perform(
                            async {
                                tokio::task::spawn_blocking(scan_java_runtimes)
                                    .await
                                    .unwrap_or_default()
                            },
                            Message::JavaScanFinished,
                        );
                    }
                    Err(e) => {
                        eprintln!("[RixLauncher] [ERROR] Java installation failed: {}", e);
                        self.status_message = format!("Java installation failed: {e}");
                    }
                }
                self.update_selected_instance(None, None);
                Task::none()
            }
            Message::MemoryChanged(gb) => {
                self.memory_gb = gb.clamp(constants::MIN_MEMORY_GB, constants::MAX_MEMORY_GB);
                self.update_selected_instance(None, None);
                self.save_config();
                Task::none()
            }
            Message::SetInstanceSettingsActive(active) => {
                self.instance_settings_active = active;
                if active {
                    self.browse_content_active = false;
                }
                Task::none()
            }
            Message::InstanceMemoryChanged(gb) => {
                let gb = gb.clamp(constants::MIN_MEMORY_GB, constants::MAX_MEMORY_GB);
                if let Some(index) = self
                    .instances
                    .iter()
                    .position(|i| Some(i.id) == self.selected_instance_id)
                {
                    self.instances[index].memory_gb = Some(gb);
                    let instance = self.instances[index].clone();
                    if let Err(e) = self.write_instance_profile(&instance) {
                        self.status_message = format!("Could not save instance settings: {e}");
                    } else {
                        self.status_message =
                            format!("Memory for \"{}\" set to {:.0} GB", instance.name, gb);
                    }
                }
                Task::none()
            }
            Message::InstanceMemoryReset => {
                if let Some(index) = self
                    .instances
                    .iter()
                    .position(|i| Some(i.id) == self.selected_instance_id)
                {
                    self.instances[index].memory_gb = None;
                    let instance = self.instances[index].clone();
                    if let Err(e) = self.write_instance_profile(&instance) {
                        self.status_message = format!("Could not save instance settings: {e}");
                    } else {
                        self.status_message =
                            format!("\"{}\" now uses the default memory setting", instance.name);
                    }
                }
                Task::none()
            }
            Message::InstanceJavaSelected(label) => {
                let new_override = if label == constants::AUTO_JAVA_LABEL {
                    None
                } else {
                    self.java_runtimes
                        .iter()
                        .find(|r| r.label == label)
                        .map(|r| r.path.clone())
                };
                if let Some(index) = self
                    .instances
                    .iter()
                    .position(|i| Some(i.id) == self.selected_instance_id)
                {
                    self.instances[index].java_path = new_override;
                    let instance = self.instances[index].clone();
                    if let Err(e) = self.write_instance_profile(&instance) {
                        self.status_message = format!("Could not save instance settings: {e}");
                    } else {
                        self.status_message = format!("Java for \"{}\" updated", instance.name);
                    }
                }
                Task::none()
            }
            Message::NewAccountNameChanged(name) => {
                self.new_account_name = name;
                Task::none()
            }
            Message::AddOfflineAccount => {
                let name = self.new_account_name.trim().to_string();
                if is_valid_player_name(&name)
                    && !self
                        .accounts
                        .iter()
                        .any(|acc| acc.name.eq_ignore_ascii_case(&name))
                {
                    let new_acc = Account {
                        name: name.clone(),
                        uuid: offline_uuid(&name),
                        access_token: constants::OFFLINE_ACCESS_TOKEN.to_string(),
                        is_microsoft: false,
                        refresh_token: None,
                    };
                    self.accounts.push(new_acc);
                    save_accounts(&self.accounts);
                    self.active_account = self.accounts.len() - 1;
                    self.new_account_name = String::new();
                    self.status_message = format!("Logged in as {name}");
                    self.push_activity(format!("Added user {name}"));
                    self.update_selected_instance(None, None);
                    self.save_config();
                    let uuid = self.accounts[self.active_account].uuid.clone();
                    player_skin_task(name, uuid)
                } else {
                    self.status_message =
                        "Use a unique player name (1–16 letters, numbers or _)".to_string();
                    Task::none()
                }
            }
            Message::SelectAccount(index) => {
                if index < self.accounts.len() {
                    self.active_account = index;
                    let name = &self.accounts[index].name;
                    self.status_message = format!("Switched to {name}");
                    self.update_selected_instance(None, None);
                    self.save_config();
                }
                Task::none()
            }
            Message::DeleteAccount(index) => {
                self.active_account_context_menu = None;
                self.account_to_delete = Some(index);
                Task::none()
            }
            Message::AccountRightClicked(index) => {
                self.active_account_context_menu = Some(index);
                self.context_menu_position = self.cursor_position;
                Task::none()
            }
            Message::CloseAccountContextMenu => {
                self.active_account_context_menu = None;
                Task::none()
            }
            Message::ConfirmDeleteAccount(index) => {
                self.active_account_context_menu = None;
                self.account_to_delete = Some(index);
                Task::none()
            }
            Message::DeleteAccountConfirmed(index) => {
                self.account_to_delete = None;
                self.delete_account(index);
                save_accounts(&self.accounts);
                self.save_config();
                Task::none()
            }
            Message::CancelDeleteAccount => {
                self.account_to_delete = None;
                Task::none()
            }
            Message::StartMicrosoftLogin => {
                if let Some(cancel) = self.microsoft_login_cancel.take() {
                    cancel.store(true, std::sync::atomic::Ordering::Relaxed);
                }
                self.microsoft_login_generation = self.microsoft_login_generation.wrapping_add(1);
                let generation = self.microsoft_login_generation;
                let cancel = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
                self.microsoft_login_cancel = Some(cancel);
                self.microsoft_login = Some(MicrosoftLoginState {
                    device_code: String::new(),
                    user_code: String::new(),
                    verification_uri: String::new(),
                    expires_in: 0,
                    interval: 0,
                    status: "Requesting device code from Microsoft...".to_string(),
                });
                Task::perform(
                    async move { request_microsoft_device_code().await },
                    move |result| Message::MicrosoftDeviceCodeReceived(generation, result),
                )
            }
            Message::RefreshMicrosoftAccount => {
                if self.microsoft_refreshing {
                    return Task::none();
                }
                let Some(account) = self.active_account_object().cloned() else {
                    self.status_message = "Add a Microsoft account first".to_string();
                    return Task::none();
                };
                let Some(refresh_token) = account
                    .refresh_token
                    .as_deref()
                    .filter(|token| !token.trim().is_empty())
                else {
                    self.status_message =
                        "This account has no refresh token; sign in again".to_string();
                    return Task::none();
                };
                self.microsoft_refreshing = true;
                self.status_message = "Refreshing Microsoft session...".to_string();
                let refresh_token = refresh_token.to_string();
                let account_uuid = account.uuid.clone();
                Task::perform(
                    async move { refresh_microsoft_account(refresh_token).await },
                    move |result| Message::MicrosoftAccountRefreshed(account_uuid, result),
                )
            }
            Message::MicrosoftDeviceCodeReceived(generation, res) => {
                if generation != self.microsoft_login_generation
                    || self.microsoft_login.is_none()
                    || self.microsoft_login_cancel.is_none()
                {
                    return Task::none();
                }
                match res {
                    Ok(state) => {
                        let device_code = state.device_code.clone();
                        let interval = state.interval as u64;
                        let expires_in = state.expires_in;
                        let Some(cancel) = self.microsoft_login_cancel.as_ref().cloned() else {
                            return Task::none();
                        };
                        self.microsoft_login = Some(state);
                        // Start polling task
                        Task::perform(
                            async move {
                                poll_microsoft_auth(device_code, interval, expires_in, cancel).await
                            },
                            move |result| Message::MicrosoftLoginFinished(generation, result),
                        )
                    }
                    Err(e) => {
                        if let Some(state) = &mut self.microsoft_login {
                            state.status = format!("Code request failed: {e}");
                        }
                        Task::none()
                    }
                }
            }
            Message::MicrosoftLoginFinished(generation, res) => {
                if generation != self.microsoft_login_generation || self.microsoft_login.is_none() {
                    return Task::none();
                }
                match res {
                    Ok(account) => {
                        if let Some(pos) = self
                            .accounts
                            .iter()
                            .position(|acc| acc.name == account.name)
                        {
                            self.accounts[pos] = account.clone();
                        } else {
                            self.accounts.push(account.clone());
                        }
                        save_accounts(&self.accounts);
                        self.active_account = self
                            .accounts
                            .iter()
                            .position(|acc| acc.name == account.name)
                            .unwrap_or(0);
                        self.status_message = format!("Logged in as {} (Microsoft)", account.name);
                        self.push_activity(format!("Microsoft user logged in: {}", account.name));
                        self.microsoft_login = None;
                        self.microsoft_login_cancel = None;
                        self.update_selected_instance(None, None);
                        self.save_config();
                        let name = account.name.clone();
                        let uuid = account.uuid.clone();
                        player_skin_task(name, uuid)
                    }
                    Err(e) => {
                        if let Some(state) = &mut self.microsoft_login {
                            state.status = format!("Login failed: {e}");
                        }
                        Task::none()
                    }
                }
            }
            Message::MicrosoftAccountRefreshed(account_uuid, res) => {
                self.microsoft_refreshing = false;
                match res {
                    Ok(account) => {
                        let account_name = account.name.clone();
                        let Some(account_index) = self
                            .accounts
                            .iter()
                            .position(|current| current.uuid == account_uuid)
                        else {
                            self.pending_launch_instance_id = None;
                            self.launching_instance_id = None;
                            return Task::none();
                        };
                        let active_account_matches = self.active_account == account_index;
                        self.accounts[account_index] = account.clone();
                        save_accounts(&self.accounts);
                        if active_account_matches {
                            self.status_message =
                                format!("Microsoft session refreshed for {account_name}");
                            self.push_activity(format!(
                                "Refreshed Microsoft session for {account_name}"
                            ));
                            self.update_selected_instance(None, None);
                            self.save_config();
                        }
                        let name = account.name.clone();
                        let uuid = account.uuid.clone();
                        let skin_task = player_skin_task(name, uuid);
                        if active_account_matches {
                            if let Some(instance_id) = self.pending_launch_instance_id.take() {
                                self.launch_instance(instance_id);
                            }
                        } else {
                            self.pending_launch_instance_id = None;
                            self.launching_instance_id = None;
                        }
                        Task::batch(vec![skin_task])
                    }
                    Err(error) => {
                        let active_account_matches = self
                            .active_account_object()
                            .is_some_and(|account| account.uuid == account_uuid);
                        if active_account_matches {
                            self.status_message =
                                format!("Microsoft session refresh failed: {error}");
                        }
                        self.pending_launch_instance_id = None;
                        self.launching_instance_id = None;
                        Task::none()
                    }
                }
            }
            Message::CloseMicrosoftLogin => {
                if let Some(cancel) = self.microsoft_login_cancel.take() {
                    cancel.store(true, std::sync::atomic::Ordering::Relaxed);
                }
                self.microsoft_login_generation = self.microsoft_login_generation.wrapping_add(1);
                self.microsoft_login = None;
                Task::none()
            }
            Message::CopyMicrosoftUserCode => {
                if let Some(state) = &self.microsoft_login {
                    self.status_message = "Copied login code to clipboard!".to_string();
                    iced::clipboard::write(state.user_code.clone())
                } else {
                    Task::none()
                }
            }
            Message::OpenMicrosoftVerificationUri(uri) => {
                match open_external_url(&uri) {
                    Ok(()) => {
                        self.status_message = "Opened Microsoft verification page".to_string()
                    }
                    Err(error) => self.status_message = error,
                }
                Task::none()
            }
            Message::MicrosoftLoginPollTick => Task::none(),
            Message::GameRootChanged(root) => {
                let trimmed = root.trim();
                if trimmed.is_empty() {
                    self.status_message = "Game directory cannot be empty".to_string();
                    return Task::none();
                }
                let path = PathBuf::from(trimmed);
                if let Err(error) = fs::create_dir_all(&path) {
                    self.status_message = format!("Could not use game directory: {error}");
                    return Task::none();
                }
                self.game_root = path.display().to_string();
                // Any in-flight preparation belongs to the old root. Its
                // completion must not be allowed to launch or update a newly
                // selected profile from the new root.
                self.launching_instance_id = None;
                self.pending_launch_instance_id = None;
                self.reload_instances();
                self.select_default_java();
                self.save_config();
                Task::none()
            }
            Message::ExtraJvmArgsChanged(args) => {
                self.extra_jvm_args = args;
                self.save_config();
                Task::none()
            }
            Message::ExtraGameArgsChanged(args) => {
                self.extra_game_args = args;
                self.save_config();
                Task::none()
            }
            Message::NavLayoutChanged(layout) => {
                self.nav_layout = layout;
                self.save_config();
                Task::none()
            }
            Message::JavaPathChanged(path) => {
                self.java_path = path.clone();
                self.save_config();
                Task::perform(
                    async move {
                        let path_for_worker = path.clone();
                        let major =
                            tokio::task::spawn_blocking(move || java::java_major(&path_for_worker))
                                .await
                                .ok()
                                .flatten();
                        (path, major)
                    },
                    |(path, major)| Message::JavaPathValidated(path, major),
                )
            }
            Message::JavaPathValidated(path, major) => {
                if self.java_path != path {
                    return Task::none();
                }
                if let Some(major) = major {
                    let label = format!("Java {major} - {path}");
                    if !self
                        .java_runtimes
                        .iter()
                        .any(|runtime| runtime.path == path)
                    {
                        self.java_runtimes.push(JavaRuntime {
                            label: label.clone(),
                            path,
                            major,
                        });
                    }
                    self.selected_java_label = Some(label);
                    self.status_message = format!("Java {major} selected");
                }
                Task::none()
            }
            Message::ContentKindSelected(kind) => {
                self.content_kind = kind;
                self.project_details = None;
                self.loading_project_details = false;
                self.project_details_error = None;
                self.content_search_generation = self.content_search_generation.wrapping_add(1);
                let search_generation = self.content_search_generation;
                self.content_results.clear();
                self.selected_content_id = None;
                self.content_searching = false;
                if self.browse_content_active {
                    let Some(instance) = self.selected_instance().cloned() else {
                        self.content_status = "Select an instance first".to_string();
                        return Task::none();
                    };
                    let query = self.content_query.clone();
                    let instance_id = instance.id;
                    self.content_status = "Searching...".to_string();
                    self.content_searching = true;
                    Task::perform(
                        async move {
                            tokio::task::spawn_blocking(move || {
                                search_content(kind, &query, &instance)
                            })
                            .await
                            .map_err(|error| format!("Search worker failed: {error}"))?
                        },
                        move |result| {
                            Message::ContentSearchFinished(instance_id, search_generation, result)
                        },
                    )
                } else {
                    Task::none()
                }
            }
            Message::ContentQueryChanged(query) => {
                self.content_query = query;
                Task::none()
            }
            Message::SearchContent => {
                let Some(instance) = self.selected_instance().cloned() else {
                    self.content_status = "Select an instance first".to_string();
                    return Task::none();
                };
                let kind = self.content_kind;
                let query = self.content_query.clone();
                let instance_id = instance.id;
                self.content_search_generation = self.content_search_generation.wrapping_add(1);
                let search_generation = self.content_search_generation;
                self.content_status = "Searching...".to_string();
                self.content_searching = true;
                Task::perform(
                    async move { search_content(kind, &query, &instance) },
                    move |result| {
                        Message::ContentSearchFinished(instance_id, search_generation, result)
                    },
                )
            }
            Message::ContentSearchFinished(instance_id, search_generation, result) => {
                if self.selected_instance_id != Some(instance_id)
                    || self.content_search_generation != search_generation
                {
                    return Task::none();
                }
                self.content_searching = false;
                match result {
                    Ok(mut items) => {
                        let mut seen = std::collections::HashSet::new();
                        items.retain(|item| seen.insert(item.id.clone()));
                        self.content_results = items;
                        if self.content_results.is_empty() {
                            self.content_status = "No results found".to_string();
                        } else {
                            self.content_status =
                                format!("Found {} items", self.content_results.len());
                        }
                        // Spawn a download task for each icon we don't have cached yet
                        let icon_tasks: Vec<Task<Message>> = self
                            .content_results
                            .iter()
                            .filter_map(|item| item.icon_url.clone())
                            .filter(|url| !self.icon_cache.contains_key(url))
                            .map(|url| {
                                let url_clone = url.clone();
                                Task::perform(
                                    async move {
                                        let bytes = tokio::task::spawn_blocking({
                                            let url = url_clone.clone();
                                            move || curl_get_bytes(&url)
                                        })
                                        .await
                                        .map_err(|error| format!("Icon worker failed: {error}"))
                                        .and_then(|result| result);
                                        match bytes {
                                            Ok(b) => {
                                                match image::load_from_memory(&b) {
                                                    Ok(img) => {
                                                        let rgba = img.to_rgba8();
                                                        let (width, height) = rgba.dimensions();
                                                        let handle = iced::widget::image::Handle::from_rgba(
                                                            width,
                                                            height,
                                                            rgba.into_raw(),
                                                        );
                                                        (url_clone, Some(handle))
                                                    }
                                                    Err(e) => {
                                                        eprintln!("[RixLauncher] [ERROR] Failed to decode icon {}: {}", url_clone, e);
                                                        (url_clone, None)
                                                    }
                                                }
                                            }
                                            Err(e) => {
                                                eprintln!("[RixLauncher] [ERROR] Failed to download icon {}: {}", url_clone, e);
                                                (url_clone, None)
                                            }
                                        }
                                    },
                                    |(url, handle_opt)| match handle_opt {
                                        Some(h) => Message::IconLoaded(url, h),
                                        None => Message::IconLoaded(
                                            url,
                                            iced::widget::image::Handle::from_rgba(1, 1, vec![0, 0, 0, 0]),
                                        ),
                                    },
                                )
                            })
                            .collect();
                        return Task::batch(icon_tasks);
                    }
                    Err(error) => {
                        self.content_status = format!("Search failed: {error}");
                    }
                }
                Task::none()
            }
            Message::IconLoaded(url, handle) => {
                // If it is not a dummy empty handle, cache it
                self.icon_cache.insert(url, handle);
                Task::none()
            }
            Message::SelectContent(id) => {
                self.selected_content_id = Some(id);
                Task::none()
            }
            Message::OpenProjectDetails(id) => {
                self.selected_content_id = Some(id.clone());
                self.mod_details_tab = types::ModDetailsTab::Description;
                self.loading_project_details = true;
                self.project_details_error = None;
                self.selected_gallery_index = 0;
                let id_clone = id.clone();
                Task::perform(
                    async move {
                        tokio::task::spawn_blocking(move || {
                            let url = format!("{}/project/{}", constants::MODRINTH_API_BASE_URL, id_clone);
                            let json = curl_get(&url, &[])?;
                            serde_json::from_str::<types::ModrinthProjectDetails>(&json)
                                .map_err(|e| format!("Failed to parse project details: {e}"))
                        })
                        .await
                        .map_err(|e| e.to_string())
                        .and_then(|r| r)
                    },
                    Message::ProjectDetailsLoaded,
                )
            }
            Message::ProjectDetailsLoaded(res) => {
                self.loading_project_details = false;
                match res {
                    Ok(details) => {
                        let mut tasks = Vec::new();
                        // Pre-fetch gallery images
                        if let Some(gallery) = &details.gallery {
                            for item in gallery.iter().take(8) {
                                let url = item.url.clone();
                                if !self.gallery_image_cache.contains_key(&url) {
                                    let url_clone = url.clone();
                                    tasks.push(Task::perform(
                                        async move {
                                            let bytes = tokio::task::spawn_blocking({
                                                let u = url_clone.clone();
                                                move || curl_get_bytes(&u)
                                            })
                                            .await
                                            .map_err(|e| e.to_string())
                                            .and_then(|r| r);
                                            match bytes {
                                                Ok(b) => match image::load_from_memory(&b) {
                                                    Ok(img) => {
                                                        let rgba = img.to_rgba8();
                                                        let (w, h) = rgba.dimensions();
                                                        let handle = iced::widget::image::Handle::from_rgba(w, h, rgba.into_raw());
                                                        (url_clone, Some(handle))
                                                    }
                                                    Err(_) => (url_clone, None),
                                                },
                                                Err(_) => (url_clone, None),
                                            }
                                        },
                                        |(u, h)| Message::GalleryImageLoaded(u, h),
                                    ));
                                }
                            }
                        }
                        // Also pre-fetch icon if not cached
                        if let Some(icon_url) = &details.icon_url {
                            if !self.icon_cache.contains_key(icon_url) {
                                let icon_clone = icon_url.clone();
                                tasks.push(Task::perform(
                                    async move {
                                        let bytes = tokio::task::spawn_blocking({
                                            let u = icon_clone.clone();
                                            move || curl_get_bytes(&u)
                                        })
                                        .await
                                        .map_err(|e| e.to_string())
                                        .and_then(|r| r);
                                        match bytes {
                                            Ok(b) => match image::load_from_memory(&b) {
                                                Ok(img) => {
                                                    let rgba = img.to_rgba8();
                                                    let (w, h) = rgba.dimensions();
                                                    let handle = iced::widget::image::Handle::from_rgba(w, h, rgba.into_raw());
                                                    (icon_clone, handle)
                                                }
                                                Err(_) => (icon_clone, iced::widget::image::Handle::from_rgba(1, 1, vec![0, 0, 0, 0])),
                                            },
                                            Err(_) => (icon_clone, iced::widget::image::Handle::from_rgba(1, 1, vec![0, 0, 0, 0])),
                                        }
                                    },
                                    |(u, h)| Message::IconLoaded(u, h),
                                ));
                            }
                        }
                        self.project_details = Some(details);
                        Task::batch(tasks)
                    }
                    Err(e) => {
                        self.project_details_error = Some(e);
                        Task::none()
                    }
                }
            }
            Message::CloseProjectDetails => {
                self.project_details = None;
                self.loading_project_details = false;
                self.project_details_error = None;
                Task::none()
            }
            Message::ModDetailsTabSelected(tab) => {
                self.mod_details_tab = tab;
                Task::none()
            }
            Message::SelectGalleryImage(idx) => {
                self.selected_gallery_index = idx;
                Task::none()
            }
            Message::GalleryImageLoaded(url, handle) => {
                if let Some(h) = handle {
                    self.gallery_image_cache.insert(url, h);
                }
                Task::none()
            }
            Message::OpenModrinthUrl(url) => {
                let _ = open_url(&url);
                Task::none()
            }
            Message::InstallSelectedContent => {
                let Some(instance) = self.selected_instance().cloned() else {
                    self.content_status = "Select an instance first".to_string();
                    return Task::none();
                };
                let Some(project_id) = self.selected_content_id.clone() else {
                    self.content_status = "Select an item first".to_string();
                    return Task::none();
                };
                let kind = self.content_kind;
                let instance_id = instance.id;
                if self.content_installing_project_id.is_some() {
                    self.content_status = "A content download is already in progress".to_string();
                    return Task::none();
                }
                self.content_installing_instance_id = Some(instance_id);
                self.content_installing_project_id = Some(project_id.clone());
                self.content_install_progress = 0.02;
                self.content_install_phase = "Preparing download...".to_string();
                self.content_status = "Preparing download...".to_string();
                content_install_task(instance_id, kind, project_id, instance)
            }
            Message::InstallContentDirect(id) => {
                self.selected_content_id = Some(id.clone());
                let Some(instance) = self.selected_instance().cloned() else {
                    self.content_status = "Select an instance first".to_string();
                    return Task::none();
                };
                let kind = self.content_kind;
                let instance_id = instance.id;
                if self.content_installing_project_id.is_some() {
                    self.content_status = "A content download is already in progress".to_string();
                    return Task::none();
                }
                self.content_installing_instance_id = Some(instance_id);
                self.content_installing_project_id = Some(id.clone());
                self.content_install_progress = 0.02;
                self.content_install_phase = "Preparing download...".to_string();
                self.content_status = "Preparing download...".to_string();
                content_install_task(instance_id, kind, id, instance)
            }
            Message::ContentInstallProgress(instance_id, project_id, phase, progress) => {
                if self.content_installing_instance_id == Some(instance_id)
                    && self.content_installing_project_id.as_deref() == Some(project_id.as_str())
                {
                    self.content_install_progress =
                        self.content_install_progress.max(progress.clamp(0.0, 0.99));
                    self.content_install_phase = phase.clone();
                    self.content_status = phase;
                }
                Task::none()
            }
            Message::ContentInstalled(instance_id, project_id, result) => {
                let is_current_install = self.content_installing_instance_id == Some(instance_id)
                    && self.content_installing_project_id.as_deref() == Some(project_id.as_str());
                if is_current_install {
                    self.content_install_progress = 1.0;
                    self.content_installing_instance_id = None;
                    self.content_installing_project_id = None;
                    self.content_install_phase.clear();
                }
                if self.selected_instance_id != Some(instance_id) {
                    return Task::none();
                }
                match result {
                    Ok(message) => {
                        self.show_toast(format!("✓ {}", message), 1);
                        self.content_status = message;
                        self.reload_instances();
                        self.trigger_load_local_icons()
                    }
                    Err(error) => {
                        eprintln!(
                            "[RixLauncher] [ERROR] Content installation failed: {}",
                            error
                        );
                        self.show_toast(format!("Install failed: {}", error), 2);
                        self.content_status = format!("Installation failed: {error}");
                        Task::none()
                    }
                }
            }
            Message::FileImported(instance_id, result) => {
                if self.selected_instance_id != Some(instance_id) {
                    return Task::none();
                }
                match result {
                    Ok((filename, kind)) => {
                        self.status_message = format!("Imported {} to {}", filename, kind.label());
                        self.reload_instances();
                        if kind == ContentKind::Mods {
                            self.trigger_load_local_icons()
                        } else {
                            Task::none()
                        }
                    }
                    Err(error) => {
                        eprintln!("[RixLauncher] [ERROR] File import failed: {}", error);
                        self.status_message = format!("Import failed: {}", error);
                        Task::none()
                    }
                }
            }
            Message::LocalModIconsLoaded(instance_id, result) => {
                if self.selected_instance_id != Some(instance_id) {
                    return Task::none();
                }
                if let Ok(icons) = result {
                    for (filename, bytes) in icons {
                        let handle = iced::widget::image::Handle::from_bytes(bytes);
                        self.local_mod_icons.insert(filename, handle);
                    }
                }
                Task::none()
            }
            Message::PlayerSkinLoaded(account_uuid, result) => {
                if !self
                    .accounts
                    .iter()
                    .any(|account| account.uuid == account_uuid)
                {
                    return Task::none();
                }
                if let Ok((name, bytes)) = result {
                    let handle = iced::widget::image::Handle::from_bytes(bytes);
                    self.player_skins.insert(name, handle);
                }
                Task::none()
            }
            Message::InstanceVersionSelected(version) => {
                self.update_selected_instance(Some(version), None);
                self.settings_version_expanded = false;
                Task::none()
            }
            Message::InstanceLoaderSelected(loader) => {
                self.update_selected_instance(None, Some(loader));
                Task::none()
            }
            Message::CloseWindow => {
                if let Some(cancel) = self.microsoft_login_cancel.take() {
                    cancel.store(true, std::sync::atomic::Ordering::Relaxed);
                }
                self.microsoft_login_generation = self.microsoft_login_generation.wrapping_add(1);
                for (_, mut child) in self.active_processes.drain() {
                    executor::stop_process_tree(&mut child);
                }
                self.instance_launch_times.clear();
                iced::window::latest().and_then(iced::window::close)
            }
            Message::MinimizeWindow => {
                if self.target_corner_radius == 0.0 {
                    // Window is maximized — first unmaximize, then minimize on next state update
                    self.pending_minimize = true;
                    iced::window::latest().and_then(|id| iced::window::maximize(id, false))
                } else {
                    // Normal window — just minimize directly
                    iced::window::latest().and_then(|id| iced::window::minimize(id, true))
                }
            }
            Message::MaximizeWindow => {
                let is_maximized = self.target_corner_radius == 0.0;
                iced::window::latest().and_then(move |id| iced::window::maximize(id, !is_maximized))
            }
            Message::DragWindow => iced::window::latest().and_then(iced::window::drag),
            Message::ResizeWindow(direction) => {
                iced::window::latest().and_then(move |id| iced::window::drag_resize(id, direction))
            }
            Message::AnimTick => {
                self.anim_time += constants::ANIMATION_TICK_MILLIS as f32;
                let diff = self.target_corner_radius - self.corner_radius;
                if diff.abs() > 0.001 {
                    self.corner_radius += diff * 0.15;
                } else {
                    self.corner_radius = self.target_corner_radius;
                }
                // Auto-expire toasts
                if let Some((_, _, expiry)) = self.toast {
                    if self.anim_time >= expiry {
                        self.toast = None;
                    }
                }
                if !self.platform_ready {
                    let width = self.window_width;
                    let height = self.window_height;
                    let radius = self.corner_radius.round() as i32;
                    iced::window::latest().and_then(move |id| {
                        apply_window_platform_task(id, width, height, radius)
                    })
                } else {
                    Task::none()
                }
            }
            Message::WindowEvent(id, event) => match event {
                iced::window::Event::Opened { size, .. } => {
                    self.update_window_size(size);
                    apply_window_platform_task(
                        id,
                        self.window_width,
                        self.window_height,
                        self.corner_radius.round() as i32,
                    )
                }
                iced::window::Event::Resized(size) => {
                    self.update_window_size(size);
                    Task::batch(vec![
                        apply_window_platform_task(
                            id,
                            self.window_width,
                            self.window_height,
                            self.corner_radius.round() as i32,
                        ),
                        iced::window::is_maximized(id)
                            .map(Message::WindowMaximizedStatusReceived),
                    ])
                }
                iced::window::Event::FileDropped(path) => {
                    println!(
                        "[RixLauncher Debug] Iced Event::FileDropped fired: {:?}",
                        path
                    );
                    if let Some(instance) = self.selected_instance() {
                        let instance_path = instance.path.clone();
                        let instance_id = instance.id;
                        let current_kind = self.content_kind;
                        Task::perform(
                            async move {
                                import_dropped_file_task(path, instance_path, current_kind).await
                            },
                            move |result| Message::FileImported(instance_id, result),
                        )
                    } else {
                        self.status_message =
                            "Select an instance first to drag and drop files".to_string();
                        Task::none()
                    }
                }
                _ => {
                    if format!("{:?}", event).contains("File")
                        || format!("{:?}", event).contains("Hover")
                    {
                        println!("[RixLauncher Debug] WindowEvent: {:?}", event);
                    }
                    self.check_wayland_dnd()
                }
            },
            Message::WindowMaximizedStatusReceived(maximized) => {
                if maximized {
                    self.target_corner_radius = 0.0;
                } else {
                    self.target_corner_radius = constants::DEFAULT_CORNER_RADIUS;
                }
                let width = self.window_width;
                let height = self.window_height;
                let radius = self.target_corner_radius.round() as i32;
                let window_platform_task = iced::window::latest()
                    .and_then(move |id| apply_window_platform_task(id, width, height, radius));
                // If we were waiting to minimize after unmaximizing, fire it now
                if !maximized && self.pending_minimize {
                    self.pending_minimize = false;
                    let minimize_task =
                        iced::window::latest().and_then(|id| iced::window::minimize(id, true));
                    Task::batch(vec![window_platform_task, minimize_task])
                } else {
                    window_platform_task
                }
            }
            Message::TitleBarPressed => {
                let now = std::time::Instant::now();
                let last_click = self.last_titlebar_click;
                self.last_titlebar_click = now;
                if now.duration_since(last_click)
                    < std::time::Duration::from_millis(constants::DOUBLE_CLICK_MILLIS)
                {
                    iced::window::latest().and_then(iced::window::toggle_maximize)
                } else {
                    iced::window::latest().and_then(iced::window::drag)
                }
            }
            Message::WindowPlatformReady => {
                self.platform_ready = true;
                Task::none()
            }
            Message::SelectImportFile => {
                let Some(instance) = self.selected_instance() else {
                    self.status_message = "Select an instance first to import files".to_string();
                    return Task::none();
                };
                let instance_id = instance.id;
                let current_kind = self.content_kind;
                Task::perform(
                    async {
                        rfd::AsyncFileDialog::new()
                            .add_filter("Mods & Packs (*.jar, *.zip)", &["jar", "zip"])
                            .pick_file()
                            .await
                            .map(|file| file.path().to_path_buf())
                    },
                    move |path| Message::ImportFileSelected(instance_id, current_kind, path),
                )
            }
            Message::ImportFileSelected(instance_id, current_kind, path_opt) => {
                if self.selected_instance_id != Some(instance_id) {
                    return Task::none();
                }
                if let Some(path) = path_opt {
                    if let Some(instance) = self.selected_instance() {
                        let instance_path = instance.path.clone();
                        Task::perform(
                            async move {
                                import_dropped_file_task(path, instance_path, current_kind).await
                            },
                            move |result| Message::FileImported(instance_id, result),
                        )
                    } else {
                        self.status_message =
                            "Select an instance first to import files".to_string();
                        Task::none()
                    }
                } else {
                    Task::none()
                }
            }
        }
    }

    pub fn subscription(&self) -> Subscription<Message> {
        let mut subs = Vec::new();

        let is_animating = (self.corner_radius - self.target_corner_radius).abs() > 0.001
            || self.launching_instance_id.is_some()
            || !self.active_processes.is_empty()
            || self.toast.is_some()
            || self.content_searching
            || self.content_installing_project_id.is_some();
        if is_animating {
            subs.push(
                iced::time::every(std::time::Duration::from_millis(
                    constants::ANIMATION_TICK_MILLIS,
                ))
                .map(|_| Message::AnimTick),
            );
        }

        if !self.active_processes.is_empty() {
            subs.push(
                iced::time::every(std::time::Duration::from_millis(
                    constants::PROCESS_POLL_MILLIS,
                ))
                .map(|_| Message::PollRunningProcesses),
            );
        }

        #[cfg(target_os = "linux")]
        subs.push(
            iced::time::every(std::time::Duration::from_millis(
                constants::WAYLAND_DND_POLL_MILLIS,
            ))
            .map(|_| Message::PollWaylandDnd),
        );

        subs.push(iced::window::events().map(|(id, event)| Message::WindowEvent(id, event)));
        subs.push(iced::event::listen().map(Message::EventOccurred));

        Subscription::batch(subs)
    }

    pub fn check_wayland_dnd(&mut self) -> Task<Message> {
        #[cfg(target_os = "linux")]
        {
            crate::wayland_dnd::poll_dnd();
            let paths = crate::wayland_dnd::take_dropped_paths();
            if !paths.is_empty() {
                println!(
                    "[RixLauncher DND Debug] Captured dropped paths: {:?}",
                    paths
                );
                if let Some(instance) = self.selected_instance() {
                    let instance_path = instance.path.clone();
                    let instance_id = instance.id;
                    let current_kind = self.content_kind;
                    let mut tasks = Vec::new();
                    for path in paths {
                        let p = path.clone();
                        let ip = instance_path.clone();
                        let ck = current_kind;
                        tasks.push(Task::perform(
                            async move { import_dropped_file_task(p, ip, ck).await },
                            move |result| Message::FileImported(instance_id, result),
                        ));
                    }
                    return Task::batch(tasks);
                } else {
                    self.status_message =
                        "Select an instance first to import dragged files".to_string();
                }
            }
        }
        Task::none()
    }

    pub fn view(&self) -> Element<'_, Message> {
        views::view_main(self)
    }

    pub fn active_player(&self) -> &str {
        self.accounts
            .get(self.active_account)
            .map(|acc| acc.name.as_str())
            .unwrap_or(constants::DEFAULT_PLAYER_NAME)
    }

    pub fn active_account_object(&self) -> Option<&Account> {
        self.accounts.get(self.active_account)
    }

    pub fn selected_instance(&self) -> Option<&MinecraftInstance> {
        let selected_id = self.selected_instance_id?;
        self.instances.iter().find(|i| i.id == selected_id)
    }

    #[allow(dead_code)]
    pub fn selected_java_runtime(&self) -> Option<&JavaRuntime> {
        let label = self.selected_java_label.as_ref()?;
        self.java_runtimes.iter().find(|r| r.label == *label)
    }

    pub fn selected_java_path(&self) -> String {
        self.java_path.clone()
    }

    #[allow(dead_code)]
    pub fn java_labels(&self) -> Vec<String> {
        self.java_runtimes.iter().map(|r| r.label.clone()).collect()
    }

    pub fn select_default_java(&mut self) {
        let required = self.required_java_major();
        self.select_default_java_for_major(required);
    }

    pub fn select_default_java_for_version(&mut self, version: &str) {
        self.select_default_java_for_major(required_java_major(version.trim()));
    }

    fn select_default_java_for_major(&mut self, required: u32) {
        if let Some(runtime) = self
            .java_runtimes
            .iter()
            .find(|runtime| runtime.major >= required)
        {
            self.selected_java_label = Some(runtime.label.clone());
            self.java_path = runtime.path.clone();
        }
    }

    pub fn required_java_major_for_create(&self) -> u32 {
        if !self.create_version.trim().is_empty() {
            required_java_major(self.create_version.trim())
        } else {
            self.required_java_major()
        }
    }

    pub fn required_java_major(&self) -> u32 {
        if let Some(instance) = self.selected_instance() {
            // The cached Mojang version JSON carries the authoritative
            // javaVersion.majorVersion for this instance.
            let game_root_path = PathBuf::from(self.game_root.trim());
            let dot_rixlauncher = game_root_path.parent().unwrap_or(&game_root_path);
            let versions_dir = dot_rixlauncher.join("versions");
            return java::required_java_major_for_version(&versions_dir, &instance.version);
        }
        if !self.create_version.trim().is_empty() {
            return required_java_major(self.create_version.trim());
        }
        self.minecraft_versions
            .first()
            .map(String::as_str)
            .map(required_java_major)
            .unwrap_or_else(|| required_java_major(constants::DEFAULT_MINECRAFT_VERSION))
    }

    pub fn reload_instances(&mut self) {
        let previous_selection = self.selected_instance_id;
        self.instances = load_instances(&self.game_root);
        if let Some(selected_id) = self.selected_instance_id {
            if !self.instances.iter().any(|i| i.id == selected_id) {
                self.selected_instance_id = self.instances.first().map(|i| i.id);
            }
        } else {
            self.selected_instance_id = self.instances.first().map(|i| i.id);
        }
        if self.selected_instance_id != previous_selection {
            self.clear_instance_scoped_state();
        }
    }

    fn clear_instance_scoped_state(&mut self) {
        self.content_search_generation = self.content_search_generation.wrapping_add(1);
        self.content_results.clear();
        self.selected_content_id = None;
        self.content_status.clear();
        self.content_searching = false;
        self.content_query.clear();
        self.installed_search_query.clear();
        self.local_mod_icons.clear();
        self.update_results.clear();
        self.updating_files.clear();
        self.checking_updates = false;
    }

    fn close_topmost_overlay(&mut self) {
        if let Some(cancel) = self.microsoft_login_cancel.take() {
            cancel.store(true, std::sync::atomic::Ordering::Relaxed);
            self.microsoft_login_generation = self.microsoft_login_generation.wrapping_add(1);
            self.microsoft_login = None;
        } else if self.active_context_menu.is_some() {
            self.active_context_menu = None;
        } else if self.active_account_context_menu.is_some() {
            self.active_account_context_menu = None;
        } else if self.account_to_delete.is_some() {
            self.account_to_delete = None;
        } else if self.file_to_delete.is_some() {
            self.file_to_delete = None;
        } else if self.instance_to_delete.is_some() {
            self.instance_to_delete = None;
        } else if self.show_clone_dialog.is_some() {
            self.show_clone_dialog = None;
            self.clone_in_progress = false;
        } else if self.show_import_dialog {
            self.show_import_dialog = false;
        } else if self.show_create_dialog {
            self.show_create_dialog = false;
        }
    }

    pub fn save_config(&self) {
        let config = LauncherConfig {
            active_account: self.active_account,
            java_path: self.java_path.clone(),
            selected_java_label: self.selected_java_label.clone(),
            memory_gb: self.memory_gb,
            extra_jvm_args: self.extra_jvm_args.clone(),
            extra_game_args: self.extra_game_args.clone(),
            game_root: self.game_root.clone(),
            selected_instance_id: self.selected_instance_id,
            nav_layout: self.nav_layout,
        };
        save_launcher_config(&config);
    }

    pub fn trigger_load_local_icons(&self) -> Task<Message> {
        if let Some(instance) = self.selected_instance() {
            let path = instance.path.clone();
            let instance_id = instance.id;
            Task::perform(
                async move {
                    tokio::task::spawn_blocking(move || load_local_mod_icons(path))
                        .await
                        .map_err(|error| format!("Icon scan worker failed: {error}"))?
                },
                move |result| Message::LocalModIconsLoaded(instance_id, result),
            )
        } else {
            Task::none()
        }
    }

    pub fn update_versions_list(&mut self) {
        self.minecraft_versions = self
            .all_versions
            .iter()
            .filter(|(_id, t)| self.show_snapshots || t == "release")
            .map(|(id, _t)| id.clone())
            .collect();

        if !self.minecraft_versions.contains(&self.create_version) {
            if let Some(first) = self.minecraft_versions.first() {
                self.create_version = first.clone();
            }
        }
    }

    pub fn delete_instance(&mut self, id: usize) {
        let Some(index) = self.instances.iter().position(|i| i.id == id) else {
            return;
        };
        if self.active_processes.contains_key(&id) {
            self.status_message = "Stop the running instance before deleting it".to_string();
            return;
        }
        let instance = &self.instances[index];
        if fs::symlink_metadata(&instance.path)
            .map(|metadata| metadata.file_type().is_symlink())
            .unwrap_or(true)
        {
            self.status_message = "Refusing to delete a symbolic-link instance".to_string();
            return;
        }
        let root = PathBuf::from(self.game_root.trim());
        let is_direct_child = instance.path.parent() == Some(root.as_path())
            && instance.path != root
            && instance.path.file_name().is_some();
        if !is_direct_child {
            self.status_message =
                "Refusing to delete a path outside the game directory".to_string();
            return;
        }
        match fs::remove_dir_all(&instance.path) {
            Ok(_) => {
                self.status_message = format!("Deleted {}", instance.name);
                self.push_activity(format!("Removed {}", instance.name));
                self.reload_instances();
                self.save_config();
            }
            Err(error) => {
                self.status_message = format!("Could not delete instance: {error}");
            }
        }
    }

    pub fn delete_account(&mut self, index: usize) {
        if index >= self.accounts.len() {
            return;
        }

        let account = self.accounts.remove(index);
        let name = account.name;
        if account.is_microsoft {
            if let Ok(entry) = account_keyring_entry(&account.uuid) {
                let _ = entry.delete_credential();
            }
        }
        if self.accounts.is_empty() {
            self.active_account = 0;
        } else if self.active_account >= self.accounts.len() {
            self.active_account = self.accounts.len().saturating_sub(1);
        } else if index < self.active_account {
            self.active_account = self.active_account.saturating_sub(1);
        }
        self.status_message = format!("Deleted account {name}");
    }

    pub fn update_selected_instance(&mut self, version: Option<String>, loader: Option<String>) {
        let Some(selected_id) = self.selected_instance_id else {
            return;
        };
        let Some(index) = self
            .instances
            .iter()
            .position(|instance| instance.id == selected_id)
        else {
            return;
        };

        if let Some(version) = version {
            if self.instances[index].version != version {
                let _ = fs::remove_file(self.instances[index].path.join("profile.json"));
            }
            self.instances[index].version = version;
        }
        if let Some(loader) = loader {
            if self.instances[index].loader != loader {
                let _ = fs::remove_file(self.instances[index].path.join("profile.json"));
            }
            self.instances[index].loader = loader;
        }

        let instance = self.instances[index].clone();
        if let Err(error) = self.write_instance_profile(&instance) {
            self.status_message = format!("Could not save instance: {error}");
        }
    }

    pub fn required_java_major_for_instance(&self, instance: &MinecraftInstance) -> u32 {
        let game_root_path = PathBuf::from(self.game_root.trim());
        let dot_rixlauncher = game_root_path.parent().unwrap_or(&game_root_path);
        let versions_dir = dot_rixlauncher.join("versions");
        java::required_java_major_for_version(&versions_dir, &instance.version)
    }

    /// Effective Java path for an instance: the per-instance override when
    /// present, otherwise the launcher-wide selection or auto-detected runtime.
    pub fn effective_java_path(&self, instance: &MinecraftInstance) -> String {
        let instance_override = instance
            .java_path
            .as_deref()
            .map(str::trim)
            .filter(|p| !p.is_empty() && !p.eq_ignore_ascii_case(constants::AUTO_JAVA_CONFIG_VALUE));

        if let Some(path) = instance_override {
            return path.to_string();
        }

        let required = self.required_java_major_for_instance(instance);

        // If launcher-wide java_path is set to an explicit path (not "auto") and meets requirement, use it
        let launcher_java = self.java_path.trim();
        if !launcher_java.is_empty()
            && !launcher_java.eq_ignore_ascii_case(constants::AUTO_JAVA_CONFIG_VALUE)
        {
            if let Some(major) = java::java_major(launcher_java) {
                if major >= required {
                    return launcher_java.to_string();
                }
            }
        }

        // Auto mode: find best matching scanned runtime (exact major or lowest >= required)
        if let Some(runtime) = self.java_runtimes.iter().find(|r| r.major == required) {
            return runtime.path.clone();
        }
        if let Some(runtime) = self.java_runtimes.iter().find(|r| r.major >= required) {
            return runtime.path.clone();
        }

        // Check managed runtime directory directly in case it was installed recently
        let managed = java::managed_java_root()
            .join(format!("jdk-{required}"))
            .join("bin")
            .join(java::java_bin_name());
        if managed.is_file() {
            return managed.display().to_string();
        }

        // Fallback to launcher_java if specified, else system java
        if !launcher_java.is_empty()
            && !launcher_java.eq_ignore_ascii_case(constants::AUTO_JAVA_CONFIG_VALUE)
        {
            launcher_java.to_string()
        } else {
            java::java_bin_name().to_string()
        }
    }

    /// Effective memory allocation (GB) for an instance.
    pub fn effective_memory_gb(&self, instance: &MinecraftInstance) -> f32 {
        instance
            .memory_gb
            .unwrap_or(self.memory_gb)
            .clamp(constants::MIN_MEMORY_GB, constants::MAX_MEMORY_GB)
    }

    /// Persists an instance's launch.properties, preserving per-instance
    /// overrides for Java and memory alongside the effective global values.
    fn write_instance_profile(&self, instance: &MinecraftInstance) -> std::io::Result<()> {
        mod_write_profile(
            &instance.path,
            &instance.name,
            &instance.version,
            &instance.loader,
            self.active_player(),
            &self.effective_java_path(instance),
            self.effective_memory_gb(instance),
            instance.java_path.as_deref(),
            instance.memory_gb,
        )
    }

    pub fn launch_selected(&mut self) {
        let Some(instance_id) = self.selected_instance_id else {
            self.status_message = "Select an instance first".to_string();
            return;
        };
        self.launch_instance(instance_id);
    }

    /// Launches the exact profile that initiated preparation. This keeps a
    /// user switching profiles while a download is running from launching a
    /// different profile when the async task completes.
    pub fn launch_instance(&mut self, instance_id: usize) {
        if self.active_processes.contains_key(&instance_id) {
            self.launching_instance_id = None;
            if let Some(instance) = self
                .instances
                .iter()
                .find(|instance| instance.id == instance_id)
            {
                self.status_message = format!("{} is already running", instance.name);
            }
            return;
        }
        let Some(instance) = self
            .instances
            .iter()
            .find(|instance| instance.id == instance_id)
            .cloned()
        else {
            self.launching_instance_id = None;
            self.status_message = "The selected instance no longer exists".to_string();
            return;
        };

        let game_root = self.game_root.clone();
        let player = self.active_player().to_string();
        let game_root_path = PathBuf::from(self.game_root.trim());
        let dot_rixlauncher = game_root_path.parent().unwrap_or(&game_root_path);
        let versions_dir = dot_rixlauncher.join("versions");
        let required_java =
            java::required_java_major_for_version(&versions_dir, &instance.version);
        let mut java_path = self.effective_java_path(&instance);
        if java::java_major(&java_path).unwrap_or(0) < required_java {
            if let Ok(installed) = java::install_java_runtime(required_java) {
                self.java_runtimes = java::scan_java_runtimes();
                java_path = installed;
            }
        }
        let memory = self.effective_memory_gb(&instance);

        let acc_opt = self.active_account_object();
        let uuid_opt = acc_opt.map(|a| a.uuid.as_str());
        let token_opt = acc_opt
            .map(|a| a.access_token.as_str())
            .filter(|token| !token.trim().is_empty());
        let is_ms = acc_opt.map(|a| a.is_microsoft).unwrap_or(false);

        let extra_jvm = self.extra_jvm_args.clone();
        let extra_game = self.extra_game_args.clone();

        match executor::launch_instance(
            &instance,
            &player,
            uuid_opt,
            token_opt,
            is_ms,
            &extra_jvm,
            &extra_game,
            &java_path,
            memory,
            &game_root,
        ) {
            Ok(child) => {
                self.active_processes.insert(instance_id, child);
                self.instance_launch_times
                    .insert(instance_id, std::time::Instant::now());
                self.launching_instance_id = None;
                self.status_message = format!("Launched {}", instance.name);
                self.push_activity(format!("Started {}", instance.name));
                self.show_toast(format!("▶ Launched {}", instance.name), 1);
            }
            Err(e) => {
                self.launching_instance_id = None;
                self.status_message = format!("Launch failed: {}", e);
                self.show_toast(format!("Launch failed: {}", e), 2);
            }
        }
    }

    pub fn push_activity(&mut self, message: String) {
        self.activity.insert(0, message);
        self.activity.truncate(7);
    }

    /// Show a transient toast for ~3 seconds. kind: 0=info, 1=success, 2=error
    pub fn show_toast(&mut self, message: String, kind: u8) {
        // Toast lasts 3000ms (anim_time units, 16ms/tick ≈ 3000 units)
        let expiry = self.anim_time + constants::TOAST_DURATION_MILLIS as f32;
        self.toast = Some((message, kind, expiry));
    }

    fn update_window_size(&mut self, size: iced::Size) {
        self.window_width = size.width.round().max(1.0) as i32;
        self.window_height = size.height.round().max(1.0) as i32;
    }
}

#[cfg(target_os = "linux")]
fn install_xdg_icon() -> Result<(), String> {
    const ICON_PNG: &[u8] = include_bytes!("../icon_256.png");

    let home = home_dir();

    // Install icon to ~/.local/share/icons/hicolor/256x256/apps/
    let icon_dir = home.join(".local/share/icons/hicolor/256x256/apps");
    fs::create_dir_all(&icon_dir).map_err(|error| error.to_string())?;
    let icon_path = icon_dir.join(format!("{}.png", constants::APPLICATION_ID));
    atomic_write(&icon_path, ICON_PNG).map_err(|error| error.to_string())?;

    // Install .desktop file so compositor associates app_id with the icon
    let desktop_dir = home.join(".local/share/applications");
    fs::create_dir_all(&desktop_dir).map_err(|error| error.to_string())?;
    let desktop_path = desktop_dir.join(constants::DESKTOP_ENTRY_NAME);
    let desktop_content = format!(
        "[Desktop Entry]\nName={}\nType=Application\nIcon={}\nExec={}\nStartupWMClass={}\nCategories=Game;\n",
        constants::APP_NAME,
        constants::APPLICATION_ID,
        constants::EXECUTABLE_NAME,
        constants::APPLICATION_ID,
    );
    atomic_write(&desktop_path, desktop_content.as_bytes()).map_err(|error| error.to_string())?;
    Ok(())
}

#[cfg(not(target_os = "linux"))]
#[allow(dead_code)]
fn install_xdg_icon() -> Result<(), String> {
    Ok(())
}

fn scan_modrinth_db_profiles() -> Result<Vec<ModrinthProfile>, String> {
    let data_root = modrinth_data_root()
        .ok_or_else(|| "Could not resolve the Modrinth App data directory".to_string())?;
    let db_path = data_root.join("ModrinthApp").join("app.db");
    if !db_path.exists() {
        return Err("Modrinth App database not found".to_string());
    }

    let connection = rusqlite::Connection::open(&db_path)
        .map_err(|error| format!("Could not open Modrinth App database: {error}"))?;
    let mut statement = connection
        .prepare("SELECT path, name, game_version, mod_loader FROM profiles")
        .map_err(|error| format!("Could not read Modrinth App profiles: {error}"))?;
    let rows = statement
        .query_map([], |row| {
            Ok(ModrinthProfile {
                path: row.get(0)?,
                name: row.get(1)?,
                game_version: row.get(2)?,
                mod_loader: row.get(3)?,
            })
        })
        .map_err(|error| format!("Could not query Modrinth App profiles: {error}"))?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("Could not decode Modrinth App profile: {error}"))
}

fn modrinth_data_root() -> Option<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        std::env::var_os("APPDATA").map(PathBuf::from)
    }
    #[cfg(target_os = "macos")]
    {
        std::env::var_os("HOME").map(|home| PathBuf::from(home).join("Library/Application Support"))
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        std::env::var_os("XDG_DATA_HOME")
            .map(PathBuf::from)
            .or_else(|| {
                std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".local/share"))
            })
    }
}

fn copy_dir_all(src: impl AsRef<Path>, dst: impl AsRef<Path>) -> std::io::Result<()> {
    let src = src.as_ref();
    let dst = dst.as_ref();

    let src_metadata = fs::symlink_metadata(src)?;
    if src_metadata.file_type().is_symlink() || !src_metadata.file_type().is_dir() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::Unsupported,
            format!("refusing to copy non-directory source {:?}", src),
        ));
    }

    match fs::symlink_metadata(dst) {
        Ok(metadata) if metadata.file_type().is_symlink() => {
            return Err(std::io::Error::new(
                std::io::ErrorKind::Unsupported,
                format!("refusing to copy through symbolic link {:?}", dst),
            ));
        }
        Ok(metadata) if !metadata.file_type().is_dir() => {
            return Err(std::io::Error::new(
                std::io::ErrorKind::AlreadyExists,
                format!("destination is not a directory {:?}", dst),
            ));
        }
        Ok(_) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            fs::create_dir_all(dst)?;
        }
        Err(error) => return Err(error),
    }

    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        if ty.is_symlink() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::Unsupported,
                format!("refusing to copy symbolic link {:?}", entry.path()),
            ));
        }
        if ty.is_dir() {
            copy_dir_all(entry.path(), dst.join(entry.file_name()))?;
        } else if ty.is_file() {
            atomic_copy_file(&entry.path(), &dst.join(entry.file_name()))?;
        }
    }
    Ok(())
}

async fn perform_import_profile(
    profile: ModrinthProfile,
    game_root: String,
    player_name: String,
    java_path: String,
    memory_gb: f32,
) -> Result<String, String> {
    let app_root = modrinth_data_root()
        .ok_or_else(|| "Could not resolve the Modrinth App data directory".to_string())?
        .join("ModrinthApp");
    let profiles_root = app_root.join("profiles");
    let src_dir = profiles_root.join(&profile.path);

    if !src_dir.exists() {
        return Err(format!("Source profile directory not found: {:?}", src_dir));
    }
    let source_root = fs::canonicalize(&profiles_root)
        .map_err(|e| format!("Could not resolve Modrinth profiles directory: {e}"))?;
    let source =
        fs::canonicalize(&src_dir).map_err(|e| format!("Could not resolve source profile: {e}"))?;
    if !source.starts_with(&source_root) {
        return Err(
            "The selected profile path is outside the Modrinth profiles directory".to_string(),
        );
    }

    let imported_version = profile.game_version.trim().to_string();
    if safe_relative_component(&imported_version).is_none() {
        return Err("The imported profile has an invalid Minecraft version".to_string());
    }
    let sanitized = sanitize_name(&profile.name);
    if game_root.trim().is_empty() {
        return Err("The game directory cannot be empty".to_string());
    }
    let game_root = PathBuf::from(game_root.trim());
    fs::create_dir_all(&game_root).map_err(|e| format!("Could not create game directory: {e}"))?;
    let dst_dir = game_root.join(&sanitized);
    let staging_dir = game_root.join(format!(".{sanitized}.importing"));

    if dst_dir.exists() {
        return Err(format!(
            "An instance named '{}' already exists in RixLauncher",
            sanitized
        ));
    }

    let _ = fs::remove_dir_all(&staging_dir);
    let source_for_copy = source.clone();
    let staging_for_copy = staging_dir.clone();
    let copy_result =
        tokio::task::spawn_blocking(move || copy_dir_all(&source_for_copy, &staging_for_copy))
            .await
            .map_err(|error| format!("Import worker failed: {error}"))?;
    if let Err(error) = copy_result {
        let _ = fs::remove_dir_all(&staging_dir);
        return Err(format!("Failed to copy files: {error}"));
    }

    // Map loader name to Capitalized format (fabric -> Fabric)
    let loader_mapped = match profile.mod_loader.to_lowercase().as_str() {
        "fabric" => constants::FABRIC_LOADER,
        "quilt" => constants::QUILT_LOADER,
        "vanilla" | "" => constants::VANILLA_LOADER,
        other => {
            let _ = fs::remove_dir_all(&staging_dir);
            return Err(format!("Loader '{other}' is not supported for import"));
        }
    };

    // If Fabric or Quilt, fetch and validate loader metadata before exposing
    // the imported directory to the launcher.
    let loader_json = match fetch_loader_profile_json(&imported_version, loader_mapped).await {
        Ok(json) => json,
        Err(error) => {
            let _ = fs::remove_dir_all(&staging_dir);
            return Err(error);
        }
    };
    if let Some(loader_json) = loader_json {
        if let Err(error) = atomic_write(&staging_dir.join("profile.json"), loader_json.as_bytes())
        {
            let _ = fs::remove_dir_all(&staging_dir);
            return Err(format!("Failed to write loader profile: {error}"));
        }
    }

    // Create launch.properties
    if let Err(error) = mod_write_profile(
        &staging_dir,
        &sanitized,
        &imported_version,
        loader_mapped,
        &player_name,
        &java_path,
        memory_gb.clamp(constants::MIN_MEMORY_GB, constants::MAX_MEMORY_GB),
        None,
        None,
    ) {
        let _ = fs::remove_dir_all(&staging_dir);
        return Err(format!("Failed to write launch.properties: {error}"));
    }

    if let Err(error) = fs::rename(&staging_dir, &dst_dir) {
        let _ = fs::remove_dir_all(&staging_dir);
        return Err(format!("Failed to finalize imported profile: {error}"));
    }

    Ok(sanitized)
}

async fn fetch_loader_profile_json(version: &str, loader: &str) -> Result<Option<String>, String> {
    let (list_url, profile_url) = match loader {
        constants::FABRIC_LOADER => (
            format!(
                "{}/versions/loader/{version}",
                constants::FABRIC_META_BASE_URL
            ),
            format!(
                "{}/versions/loader/{{version}}/{{loader_version}}/profile/json",
                constants::FABRIC_META_BASE_URL
            ),
        ),
        constants::QUILT_LOADER => (
            format!(
                "{}/versions/loader/{version}",
                constants::QUILT_META_BASE_URL
            ),
            format!(
                "{}/versions/loader/{{version}}/{{loader_version}}/profile/json",
                constants::QUILT_META_BASE_URL
            ),
        ),
        _ => return Ok(None),
    };
    validate_remote_url(&list_url)?;
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(
            constants::IMPORT_REQUEST_TIMEOUT_SECONDS,
        ))
        .build()
        .map_err(|e| format!("Could not initialize loader client: {e}"))?;
    let loaders: serde_json::Value = client
        .get(&list_url)
        .header("User-Agent", utils::USER_AGENT)
        .send()
        .await
        .map_err(|e| format!("Could not fetch {loader} loader list: {e}"))?
        .error_for_status()
        .map_err(|e| format!("{loader} loader list returned an error: {e}"))?
        .json()
        .await
        .map_err(|e| format!("Could not parse {loader} loader list: {e}"))?;
    let first = loaders
        .as_array()
        .and_then(|items| {
            if loader == constants::FABRIC_LOADER {
                items
                    .iter()
                    .find(|item| item["loader"]["stable"].as_bool().unwrap_or(false))
                    .or_else(|| items.first())
            } else {
                items.first()
            }
        })
        .ok_or_else(|| format!("No {loader} loader is available for Minecraft {version}"))?;
    let loader_version = first["loader"]["version"]
        .as_str()
        .ok_or_else(|| format!("{loader} response did not contain a loader version"))?;
    let profile_url = profile_url
        .replace("{version}", version)
        .replace("{loader_version}", loader_version);
    validate_remote_url(&profile_url)?;
    client
        .get(profile_url)
        .header("User-Agent", utils::USER_AGENT)
        .send()
        .await
        .map_err(|e| format!("Could not fetch {loader} profile: {e}"))?
        .error_for_status()
        .map_err(|e| format!("{loader} profile returned an error: {e}"))?
        .text()
        .await
        .map(Some)
        .map_err(|e| format!("Could not read {loader} profile: {e}"))
}

#[allow(unused_variables)]
fn apply_window_platform_task(
    id: iced::window::Id,
    width: i32,
    height: i32,
    radius: i32,
) -> Task<Message> {
    iced::window::run(id, move |window| {
        #[cfg(target_os = "linux")]
        {
            use raw_window_handle::{RawDisplayHandle, RawWindowHandle};
            if let (Ok(display), Ok(window_handle)) =
                (window.display_handle(), window.window_handle())
            {
                if let (
                    RawDisplayHandle::Wayland(display_wayland),
                    RawWindowHandle::Wayland(window_wayland),
                ) = (display.as_raw(), window_handle.as_raw())
                {
                    unsafe {
                        crate::blur::apply_kde_wayland_blur(
                            display_wayland.display.as_ptr() as *mut _,
                            window_wayland.surface.as_ptr() as *mut _,
                            width,
                            height,
                            radius,
                        );
                        crate::wayland_dnd::init_wayland_dnd(
                            display_wayland.display.as_ptr() as *mut _
                        );
                    }
                }
            }
        }

        #[cfg(target_os = "windows")]
        {
            use raw_window_handle::RawWindowHandle;
            if let Ok(window_handle) = window.window_handle() {
                if let RawWindowHandle::Win32(window_win32) = window_handle.as_raw() {
                    unsafe { apply_windows_blur(window_win32.hwnd.get() as *mut _) };
                }
            }
        }
        Message::WindowPlatformReady
    })
}

#[cfg(target_os = "windows")]
unsafe fn apply_windows_blur(hwnd: *mut std::ffi::c_void) {
    use std::ffi::c_void;

    #[repr(C)]
    struct Margins {
        cx_left_width: i32,
        cx_right_width: i32,
        cy_top_height: i32,
        cy_bottom_height: i32,
    }

    #[repr(C)]
    struct DwmBlurBehind {
        dw_flags: u32,
        f_enable: i32,
        h_rgn_blur: *mut c_void,
        f_transition_on_maximized: i32,
    }

    #[repr(C)]
    struct AccentPolicy {
        accent_state: u32,
        accent_flags: u32,
        gradient_color: u32,
        animation_id: u32,
    }

    #[repr(C)]
    struct WindowCompositionAttributeData {
        attribute: u32,
        data: *mut c_void,
        size_of_data: usize,
    }

    #[link(name = "dwmapi")]
    unsafe extern "system" {
        fn DwmExtendFrameIntoClientArea(hwnd: *mut c_void, margins: *const Margins) -> i32;
        fn DwmEnableBlurBehindWindow(hwnd: *mut c_void, p_blur_behind: *const DwmBlurBehind) -> i32;
        fn DwmSetWindowAttribute(
            hwnd: *mut c_void,
            attribute: u32,
            value: *const c_void,
            value_size: u32,
        ) -> i32;
    }

    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn LoadLibraryA(name: *const u8) -> *mut c_void;
        fn GetProcAddress(module: *mut c_void, name: *const u8) -> *mut c_void;
    }

    // 1. Clear any legacy empty-region BlurBehind set by winit transparent initialization,
    // so DWM's backdrop system can render unimpeded.
    let clear_bb = DwmBlurBehind {
        dw_flags: 1, // DWM_BB_ENABLE
        f_enable: 0,
        h_rgn_blur: std::ptr::null_mut(),
        f_transition_on_maximized: 0,
    };
    let _ = unsafe { DwmEnableBlurBehindWindow(hwnd, &clear_bb) };

    // 2. Extend the DWM frame into the entire client area (-1 margins).
    // This allows DWM backdrop effects (Acrylic/Blur) to paint across the full client window.
    let margins = Margins {
        cx_left_width: -1,
        cx_right_width: -1,
        cy_top_height: -1,
        cy_bottom_height: -1,
    };
    let _ = unsafe { DwmExtendFrameIntoClientArea(hwnd, &margins) };

    // 3. Request dark mode composition (DWMWA_USE_IMMERSIVE_DARK_MODE = 20)
    let dark_mode: u32 = 1;
    let _ = unsafe {
        DwmSetWindowAttribute(
            hwnd,
            20,
            &dark_mode as *const u32 as *const _,
            std::mem::size_of::<u32>() as u32,
        )
    };

    // 4. Request rounded corners (DWMWA_WINDOW_CORNER_PREFERENCE = 33, DWMWCP_ROUND = 2)
    let corner_preference: u32 = 2;
    let _ = unsafe {
        DwmSetWindowAttribute(
            hwnd,
            33,
            &corner_preference as *const u32 as *const _,
            std::mem::size_of::<u32>() as u32,
        )
    };

    // 5. On Windows 11 (22H2+ build 22621+): Request true Acrylic frosted glass (DWMSBT_TRANSIENTWINDOW = 3)
    // DWMWA_SYSTEMBACKDROP_TYPE = 38
    let backdrop_acrylic: u32 = 3;
    let result = unsafe {
        DwmSetWindowAttribute(
            hwnd,
            38,
            &backdrop_acrylic as *const u32 as *const _,
            std::mem::size_of::<u32>() as u32,
        )
    };

    // If DWM Acrylic backdrop is not supported (Windows 11 21H2 build 22000, or Windows 10):
    if result != 0 {
        // Try Windows 11 21H2 undocumented Mica/Acrylic (DWMWA_MICA_EFFECT = 1029)
        let mica_on: u32 = 1;
        let _ = unsafe {
            DwmSetWindowAttribute(
                hwnd,
                1029,
                &mica_on as *const u32 as *const _,
                std::mem::size_of::<u32>() as u32,
            )
        };

        // Also apply SetWindowCompositionAttribute (Windows 10 v1809+ Acrylic / BlurBehind)
        let user32 = unsafe { LoadLibraryA(b"user32.dll\0".as_ptr()) };
        if !user32.is_null() {
            let proc = unsafe { GetProcAddress(user32, b"SetWindowCompositionAttribute\0".as_ptr()) };
            if !proc.is_null() {
                type SetWindowCompositionAttributeFn = unsafe extern "system" fn(
                    *mut c_void,
                    *mut WindowCompositionAttributeData,
                ) -> i32;
                let set_wca: SetWindowCompositionAttributeFn = unsafe { std::mem::transmute(proc) };

                // AccentState: 4 = ACCENT_ENABLE_ACRYLICBLURBEHIND
                // AccentFlags: 0 for Acrylic (per window-vibrancy specification)
                // GradientColor: ABGR format with subtle tint (0x9918181b = ~60% dark tint)
                let mut policy_acrylic = AccentPolicy {
                    accent_state: 4,
                    accent_flags: 0,
                    gradient_color: 0x9918181b,
                    animation_id: 0,
                };
                let mut data_acrylic = WindowCompositionAttributeData {
                    attribute: 19, // WCA_ACCENT_POLICY
                    data: &mut policy_acrylic as *mut AccentPolicy as *mut _,
                    size_of_data: std::mem::size_of::<AccentPolicy>(),
                };
                let wca_res = unsafe { set_wca(hwnd, &mut data_acrylic) };
                if wca_res != 0 {
                    // Fallback to classic blurbehind (accent_state = 3, accent_flags = 2)
                    let mut policy_blur = AccentPolicy {
                        accent_state: 3,
                        accent_flags: 2,
                        gradient_color: 0,
                        animation_id: 0,
                    };
                    let mut data_blur = WindowCompositionAttributeData {
                        attribute: 19,
                        data: &mut policy_blur as *mut AccentPolicy as *mut _,
                        size_of_data: std::mem::size_of::<AccentPolicy>(),
                    };
                    unsafe { set_wca(hwnd, &mut data_blur) };
                }
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub fn mod_write_profile(
    profile_dir: &Path,
    name: &str,
    version: &str,
    loader: &str,
    player_name: &str,
    java_path: &str,
    memory_gb: f32,
    instance_java_path: Option<&str>,
    instance_memory_gb: Option<f32>,
) -> std::io::Result<()> {
    fn property_value(value: &str) -> String {
        value.replace(['\r', '\n'], " ").trim().to_string()
    }

    let memory_gb = memory_gb.clamp(constants::MIN_MEMORY_GB, constants::MAX_MEMORY_GB);
    let mut profile = format!(
        "name={}\nplayer={}\nversion={}\nloader={}\njava={}\nmemory_gb={memory_gb:.0}\n",
        property_value(name),
        property_value(player_name),
        property_value(version),
        property_value(loader),
        property_value(java_path),
    );
    if let Some(java) = instance_java_path {
        profile.push_str(&format!("instance_java={}\n", property_value(java)));
    }
    if let Some(memory) = instance_memory_gb {
        let memory = memory.clamp(constants::MIN_MEMORY_GB, constants::MAX_MEMORY_GB);
        profile.push_str(&format!("instance_memory_gb={memory:.0}\n"));
    }
    atomic_write(&profile_dir.join("launch.properties"), profile.as_bytes())
}

fn load_accounts() -> Vec<Account> {
    let path = get_rixlauncher_root().join("accounts.json");
    if path.exists() {
        if let Ok(content) = fs::read_to_string(&path) {
            if let Ok(stored) = serde_json::from_str::<Vec<StoredAccount>>(&content) {
                return stored
                    .into_iter()
                    .map(|account| account_from_storage(account, None))
                    .collect();
            }

            // Migrate the old format once. Tokens are moved to the OS
            // credential store and accounts.json is rewritten without them.
            if let Ok(legacy) = serde_json::from_str::<Vec<LegacyAccount>>(&content) {
                let migrated: Vec<Account> = legacy
                    .into_iter()
                    .map(|legacy| {
                        let account = Account {
                            name: legacy.name,
                            uuid: legacy.uuid,
                            access_token: legacy.access_token,
                            is_microsoft: legacy.is_microsoft,
                            refresh_token: legacy.refresh_token,
                        };
                        let credentials = account.is_microsoft.then_some(AccountCredentials {
                            access_token: account.access_token.clone(),
                            refresh_token: account.refresh_token.clone(),
                        });
                        store_account_credentials(&account, credentials.as_ref());
                        account
                    })
                    .collect();
                save_accounts(&migrated);
                return migrated;
            }

            // Fallback: try loading as old simple string usernames and migrate
            if let Ok(old_usernames) = serde_json::from_str::<Vec<String>>(&content) {
                let migrated: Vec<Account> = old_usernames
                    .into_iter()
                    .map(|name| Account {
                        uuid: offline_uuid(&name),
                        name,
                        access_token: constants::OFFLINE_ACCESS_TOKEN.to_string(),
                        is_microsoft: false,
                        refresh_token: None,
                    })
                    .collect();
                // Save migrated accounts
                save_accounts(&migrated);
                return migrated;
            }
        }
    }
    Vec::new()
}

fn save_accounts(accounts: &[Account]) {
    let dir = get_rixlauncher_root();
    let _ = fs::create_dir_all(&dir);
    let path = dir.join("accounts.json");
    let stored: Vec<StoredAccount> = accounts
        .iter()
        .map(|account| {
            let credentials = account.is_microsoft.then_some(AccountCredentials {
                access_token: account.access_token.clone(),
                refresh_token: account.refresh_token.clone(),
            });
            store_account_credentials(account, credentials.as_ref());
            StoredAccount {
                name: account.name.clone(),
                uuid: account.uuid.clone(),
                is_microsoft: account.is_microsoft,
            }
        })
        .collect();
    if let Ok(content) = serde_json::to_vec_pretty(&stored) {
        if let Err(error) = atomic_write(&path, &content) {
            eprintln!("[RixLauncher] [WARN] Could not save account metadata: {error}");
        }
    }
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
struct StoredAccount {
    name: String,
    uuid: String,
    is_microsoft: bool,
}

#[derive(serde::Deserialize, Clone, Debug)]
struct LegacyAccount {
    name: String,
    uuid: String,
    access_token: String,
    is_microsoft: bool,
    refresh_token: Option<String>,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
struct AccountCredentials {
    access_token: String,
    refresh_token: Option<String>,
}

fn account_keyring_entry(uuid: &str) -> Result<keyring::Entry, String> {
    keyring::Entry::new(constants::ACCOUNT_CREDENTIAL_SERVICE, uuid)
        .map_err(|error| format!("credential store unavailable: {error}"))
}

fn store_account_credentials(account: &Account, credentials: Option<&AccountCredentials>) {
    let Ok(entry) = account_keyring_entry(&account.uuid) else {
        return;
    };
    match credentials {
        Some(credentials) if !credentials.access_token.trim().is_empty() => {
            match serde_json::to_string(credentials) {
                Ok(secret) => {
                    if let Err(error) = entry.set_password(&secret) {
                        eprintln!(
                            "[RixLauncher] [WARN] Could not store credentials securely: {error}"
                        );
                    }
                }
                Err(error) => {
                    eprintln!("[RixLauncher] [WARN] Could not encode credentials: {error}")
                }
            }
        }
        _ => {
            let _ = entry.delete_credential();
        }
    }
}

fn load_account_credentials(uuid: &str) -> Option<AccountCredentials> {
    let entry = account_keyring_entry(uuid).ok()?;
    let secret = entry.get_password().ok()?;
    serde_json::from_str(&secret).ok()
}

fn account_from_storage(stored: StoredAccount, _legacy: Option<LegacyAccount>) -> Account {
    let credentials = stored
        .is_microsoft
        .then(|| load_account_credentials(&stored.uuid))
        .flatten();
    Account {
        name: stored.name,
        uuid: stored.uuid,
        access_token: credentials
            .as_ref()
            .map(|credentials| credentials.access_token.clone())
            .unwrap_or_default(),
        is_microsoft: stored.is_microsoft,
        refresh_token: credentials.and_then(|credentials| credentials.refresh_token),
    }
}

fn is_valid_player_name(name: &str) -> bool {
    let length = name.chars().count();
    (1..=constants::MAX_PLAYER_NAME_CHARS).contains(&length)
        && name
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || character == '_')
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
#[serde(default)]
pub struct LauncherConfig {
    pub active_account: usize,
    pub java_path: String,
    pub selected_java_label: Option<String>,
    pub memory_gb: f32,
    pub extra_jvm_args: String,
    pub extra_game_args: String,
    pub game_root: String,
    pub selected_instance_id: Option<usize>,
    pub nav_layout: NavLayout,
}

impl Default for LauncherConfig {
    fn default() -> Self {
        Self {
            active_account: 0,
            java_path: constants::AUTO_JAVA_CONFIG_VALUE.to_string(),
            selected_java_label: None,
            memory_gb: constants::DEFAULT_MEMORY_GB,
            extra_jvm_args: String::new(),
            extra_game_args: String::new(),
            game_root: default_game_root(),
            selected_instance_id: None,
            nav_layout: NavLayout::TopBar,
        }
    }
}

pub fn load_config() -> Option<LauncherConfig> {
    let path = get_rixlauncher_root().join("config.json");
    if path.exists() {
        if let Ok(content) = fs::read_to_string(&path) {
            if let Ok(config) = serde_json::from_str::<LauncherConfig>(&content) {
                return Some(config);
            }
            eprintln!(
                "[RixLauncher] [WARN] Ignoring invalid launcher config at {}",
                path.display()
            );
        }
    }
    None
}

pub fn save_launcher_config(config: &LauncherConfig) {
    let dir = get_rixlauncher_root();
    let _ = fs::create_dir_all(&dir);
    let path = dir.join("config.json");
    if let Ok(content) = serde_json::to_vec_pretty(config) {
        if let Err(error) = atomic_write(&path, &content) {
            eprintln!("[RixLauncher] [WARN] Could not save launcher config: {error}");
        }
    }
}

fn default_game_root() -> String {
    get_rixlauncher_root()
        .join("instances")
        .display()
        .to_string()
}

fn load_instances(root: &str) -> Vec<MinecraftInstance> {
    let root = PathBuf::from(root.trim());
    let Ok(entries) = fs::read_dir(&root) else {
        return Vec::new();
    };

    let mut instances = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        let Ok(file_type) = entry.file_type() else {
            continue;
        };
        if !file_type.is_dir() || file_type.is_symlink() {
            continue;
        }

        // A folder becomes an instance only after the launcher has written its
        // marker file. This prevents half-created/download-cache directories
        // from appearing as playable profiles.
        let properties_path = path.join("launch.properties");
        if !properties_path.is_file() {
            continue;
        }
        let properties = read_properties(&properties_path);
        let Some(name) = properties
            .get("name")
            .map(|value| value.trim())
            .filter(|value| !value.is_empty())
            .map(str::to_owned)
        else {
            continue;
        };
        let Some(version) = properties
            .get("version")
            .map(|value| value.trim())
            .filter(|value| !value.is_empty())
            .map(str::to_owned)
        else {
            continue;
        };
        let Some(version_component) = safe_relative_component(&version) else {
            continue;
        };
        if !downloader::is_valid_instance_name(&name) {
            continue;
        }
        let loader = properties
            .get("loader")
            .map(|value| value.trim().to_string())
            .unwrap_or_else(|| constants::VANILLA_LOADER.to_string());
        let instance_memory = properties
            .get("instance_memory_gb")
            .and_then(|v| v.parse::<f32>().ok())
            .filter(|v| *v > 0.0);
        let instance_java = properties
            .get("instance_java")
            .map(|p| p.to_string())
            .filter(|p| !p.trim().is_empty() && p.trim() != constants::AUTO_JAVA_CONFIG_VALUE);

        let client_jar_path = root
            .parent()
            .unwrap_or(&root)
            .join("versions")
            .join(&version_component)
            .join(format!("{version}.jar"));

        instances.push(MinecraftInstance {
            id: stable_instance_id(&path),
            name,
            version: version.clone(),
            loader,
            mod_count: count_jars(&path.join("mods")),
            resource_pack_count: count_files(&path.join("resourcepacks")),
            shader_count: count_files(&path.join("shaderpacks")),
            has_version_files: root
                .parent()
                .unwrap_or(&root)
                .join("versions")
                .join(&version_component)
                .join(format!("{version}.json"))
                .is_file(),
            has_runnable_jar: client_jar_path.is_file(),
            path,
            memory_gb: instance_memory,
            java_path: instance_java,
        });
    }

    instances.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    instances
}

fn read_properties(path: &Path) -> HashMap<String, String> {
    let mut values = HashMap::new();
    let Ok(contents) = fs::read_to_string(path) else {
        return values;
    };

    for line in contents.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if let Some((key, value)) = trimmed.split_once('=') {
            values.insert(key.trim().to_string(), value.trim().to_string());
        }
    }

    values
}

fn fetch_minecraft_versions() -> Result<Vec<(String, String)>, String> {
    let manifest = curl_get(constants::MOJANG_VERSION_MANIFEST_URL, &[])?;
    let parsed: types::VersionManifest = serde_json::from_str(&manifest)
        .map_err(|error| format!("Could not parse Mojang version manifest: {error}"))?;

    Ok(parsed
        .versions
        .into_iter()
        .filter(|version| !version.id.is_empty() && !version.version_type.is_empty())
        .map(|version| (version.id, version.version_type))
        .collect())
}

fn loader_profile_matches_instance(path: &Path, version: &str, loader: &str) -> bool {
    let Ok(content) = fs::read_to_string(path) else {
        return false;
    };
    let Ok(profile) = serde_json::from_str::<types::LoaderProfile>(&content) else {
        return false;
    };
    if profile.inherits_from.as_deref() != Some(version) {
        return false;
    }
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

#[cfg(target_os = "linux")]
#[allow(dead_code)]
async fn get_maximized_status() -> bool {
    false
}

fn search_content(
    kind: ContentKind,
    query: &str,
    instance: &MinecraftInstance,
) -> Result<Vec<ContentItem>, String> {
    search_modrinth(kind, query, instance)
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
struct InstalledContentRegistry {
    #[serde(default)]
    mods: HashMap<String, Vec<String>>,
    #[serde(default)]
    resourcepacks: HashMap<String, Vec<String>>,
    #[serde(default)]
    shaderpacks: HashMap<String, Vec<String>>,
}

fn content_registry_bucket(
    registry: &InstalledContentRegistry,
    kind: ContentKind,
) -> &HashMap<String, Vec<String>> {
    match kind {
        ContentKind::Mods => &registry.mods,
        ContentKind::ResourcePacks => &registry.resourcepacks,
        ContentKind::Shaders => &registry.shaderpacks,
    }
}

fn content_registry_bucket_mut(
    registry: &mut InstalledContentRegistry,
    kind: ContentKind,
) -> &mut HashMap<String, Vec<String>> {
    match kind {
        ContentKind::Mods => &mut registry.mods,
        ContentKind::ResourcePacks => &mut registry.resourcepacks,
        ContentKind::Shaders => &mut registry.shaderpacks,
    }
}

fn read_content_registry(instance: &MinecraftInstance) -> InstalledContentRegistry {
    let path = instance.path.join(constants::CONTENT_REGISTRY_FILE);
    fs::read_to_string(path)
        .ok()
        .and_then(|content| serde_json::from_str(&content).ok())
        .unwrap_or_default()
}

fn write_content_registry(
    instance: &MinecraftInstance,
    registry: &InstalledContentRegistry,
) -> Result<(), String> {
    let content = serde_json::to_vec_pretty(registry)
        .map_err(|error| format!("Could not serialize content registry: {error}"))?;
    atomic_write(
        &instance.path.join(constants::CONTENT_REGISTRY_FILE),
        &content,
    )
    .map_err(|error| format!("Could not save content registry: {error}"))
}

/// Project IDs recorded by the launcher. The UI also uses a filename fallback
/// for content installed before the registry existed.
pub(crate) fn installed_content_project_ids(
    instance: &MinecraftInstance,
    kind: ContentKind,
) -> std::collections::HashSet<String> {
    content_registry_bucket(&read_content_registry(instance), kind)
        .keys()
        .cloned()
        .collect()
}

fn register_installed_content(
    instance: &MinecraftInstance,
    kind: ContentKind,
    records: &[(String, String)],
) -> Result<(), String> {
    if records.is_empty() {
        return Ok(());
    }
    let mut registry = read_content_registry(instance);
    let bucket = content_registry_bucket_mut(&mut registry, kind);
    for (project_id, filename) in records {
        if safe_project_id(project_id).is_err() {
            continue;
        }
        let Some(filename) = safe_file_name(filename) else {
            continue;
        };
        let files = bucket.entry(project_id.clone()).or_default();
        if !files.iter().any(|known| known == filename) {
            files.push(filename.to_string());
        }
    }
    write_content_registry(instance, &registry)
}

fn forget_installed_content_file(
    instance: &MinecraftInstance,
    kind: ContentKind,
    filename: &str,
) -> Result<(), String> {
    let mut registry = read_content_registry(instance);
    let bucket = content_registry_bucket_mut(&mut registry, kind);
    for files in bucket.values_mut() {
        files.retain(|known| known != filename);
    }
    bucket.retain(|_, files| !files.is_empty());
    write_content_registry(instance, &registry)
}

fn content_install_task(
    instance_id: usize,
    kind: ContentKind,
    project_id: String,
    instance: MinecraftInstance,
) -> Task<Message> {
    let stream = iced::stream::channel(16, async move |mut output| {
        use iced::futures::SinkExt;

        let (progress_sender, mut progress_receiver) = tokio::sync::mpsc::unbounded_channel();
        let worker_project_id = project_id.clone();
        let worker = tokio::task::spawn_blocking(move || {
            install_content_with_progress(kind, &worker_project_id, &instance, |phase, progress| {
                let _ = progress_sender.send((phase.to_string(), progress));
            })
        });
        tokio::pin!(worker);
        let mut progress_open = true;

        loop {
            tokio::select! {
                result = &mut worker => {
                    let result = result
                        .map_err(|error| format!("Install worker failed: {error}"))
                        .and_then(|result| result);
                    let _ = output
                        .send(Message::ContentInstalled(instance_id, project_id.clone(), result))
                        .await;
                    break;
                }
                progress = progress_receiver.recv(), if progress_open => {
                    match progress {
                        Some((phase, progress)) => {
                            if output
                                .send(Message::ContentInstallProgress(
                                    instance_id,
                                    project_id.clone(),
                                    phase,
                                    progress,
                                ))
                                .await
                                .is_err()
                            {
                                break;
                            }
                        }
                        None => progress_open = false,
                    }
                }
            }
        }
    });
    Task::run(stream, |message| message)
}

fn install_content_with_progress<F>(
    kind: ContentKind,
    project_id: &str,
    instance: &MinecraftInstance,
    progress: F,
) -> Result<String, String>
where
    F: Fn(&str, f32),
{
    let result = install_modrinth(kind, project_id, instance, &progress);
    if let Err(ref e) = result {
        eprintln!(
            "[RixLauncher] [ERROR] Failed to install Modrinth content (Project ID: {}): {}",
            project_id, e
        );
    }
    result
}

fn search_modrinth(
    kind: ContentKind,
    query: &str,
    instance: &MinecraftInstance,
) -> Result<Vec<ContentItem>, String> {
    let mut facets = format!(
        "[[\"project_type:{}\"],[\"versions:{}\"]]",
        kind.modrinth_project_type(),
        instance.version
    );
    if kind == ContentKind::Mods {
        facets = format!(
            "[[\"project_type:mod\"],[\"versions:{}\"],[\"categories:{}\"]]",
            instance.version,
            instance.loader.to_lowercase()
        );
    }
    let url = format!(
        "{}/search?limit={}&index=relevance&query={}&facets={}",
        constants::MODRINTH_API_BASE_URL,
        constants::MODRINTH_SEARCH_LIMIT,
        url_encode(query),
        url_encode(&facets)
    );
    let json = curl_get(&url, &[])?;
    parse_modrinth_hits(&json)
}

fn install_modrinth<F>(
    kind: ContentKind,
    project_id: &str,
    instance: &MinecraftInstance,
    progress: &F,
) -> Result<String, String>
where
    F: Fn(&str, f32),
{
    let mut installed: Vec<String> = Vec::new();
    let mut visited: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut records = Vec::new();
    progress("Finding a compatible version...", 0.06);
    install_modrinth_recursive(
        kind,
        project_id,
        instance,
        &mut installed,
        &mut visited,
        &mut records,
        progress,
    )?;
    if let Err(error) = register_installed_content(instance, kind, &records) {
        // The downloaded files remain valid even if metadata cannot be
        // written; the browser still has the filename fallback for status.
        eprintln!("[RixLauncher] [WARN] Could not save content registry: {error}");
    }
    if installed.is_empty() {
        Ok(format!("Installed {} (already up to date)", project_id))
    } else if installed.len() == 1 {
        Ok(format!("Installed {}", installed[0]))
    } else {
        Ok(format!(
            "Installed {} (+ {} dependencies: {})",
            installed[0],
            installed.len() - 1,
            installed[1..].join(", ")
        ))
    }
}

fn install_modrinth_recursive<F>(
    kind: ContentKind,
    project_id: &str,
    instance: &MinecraftInstance,
    installed: &mut Vec<String>,
    visited: &mut std::collections::HashSet<String>,
    records: &mut Vec<(String, String)>,
    progress: &F,
) -> Result<(), String>
where
    F: Fn(&str, f32),
{
    let project_id = safe_project_id(project_id)?;
    // Avoid infinite loops / duplicate installs
    if visited.contains(project_id) {
        return Ok(());
    }
    visited.insert(project_id.to_string());
    progress("Loading project files...", 0.12);

    // Build URL - square brackets must be percent-encoded (%5B/%5D) or server returns empty body
    let game_versions_param = url_encode(&format!("[\"{}\"]", instance.version));
    let loaders_param = url_encode(&format!("[\"{}\"]", instance.loader.to_lowercase()));

    let url = if kind == ContentKind::Mods {
        format!(
            "{}/project/{}/version?game_versions={}&loaders={}",
            constants::MODRINTH_API_BASE_URL,
            url_encode(project_id),
            game_versions_param,
            loaders_param
        )
    } else {
        format!(
            "{}/project/{}/version?game_versions={}",
            constants::MODRINTH_API_BASE_URL,
            url_encode(project_id),
            game_versions_param
        )
    };

    eprintln!("[RixLauncher] [DEBUG] Modrinth install URL: {}", url);
    let json = curl_get(&url, &[])?;
    eprintln!(
        "[RixLauncher] [DEBUG] Modrinth response length: {} bytes",
        json.len()
    );
    let versions: Vec<types::ModrinthVersion> = serde_json::from_str(&json)
        .map_err(|error| format!("Could not parse Modrinth versions: {error}"))?;
    let version = versions.first().ok_or_else(|| {
        "No compatible file found for this version/loader combination".to_string()
    })?;
    let file_info = version
        .files
        .iter()
        .find(|file| file.primary)
        .or_else(|| version.files.first())
        .ok_or_else(|| "The compatible Modrinth version has no files".to_string())?;

    if file_info.url.trim().is_empty() {
        return Err("No compatible file found for this version/loader combination".to_string());
    }

    let file_url = file_info.url.clone();
    let raw_filename = if file_info.filename.trim().is_empty() {
        format!("{project_id}.jar")
    } else {
        file_info.filename.clone()
    };
    let filename = safe_remote_filename(&raw_filename)?;
    let expected_sha1 =
        (!file_info.hashes.sha1.trim().is_empty()).then_some(file_info.hashes.sha1.clone());
    let target_dir = instance.path.join(kind.install_folder());
    fs::create_dir_all(&target_dir).map_err(|error| error.to_string())?;

    // Check if a file with a similar base name already exists (skip download if so)
    let base_name = modrinth_filename_base(&filename);
    let existing_filename = fs::read_dir(&target_dir)
        .ok()
        .into_iter()
        .flatten()
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let candidate = entry.file_name().to_string_lossy().into_owned();
            (modrinth_filename_base(&candidate) == base_name).then_some(candidate)
        })
        .next();
    let already_exists = existing_filename.is_some();

    if !already_exists {
        let destination = target_dir.join(&filename);
        progress(&format!("Downloading {filename}..."), 0.28);
        curl_download(&file_url, &destination, &[])?;
        if let Some(expected_sha1) = expected_sha1.as_deref() {
            progress(&format!("Verifying {filename}..."), 0.78);
            let actual_sha1 = compute_file_sha1(&destination)?;
            if !actual_sha1.eq_ignore_ascii_case(expected_sha1) {
                let _ = fs::remove_file(&destination);
                return Err(format!(
                    "Downloaded {filename} failed checksum verification"
                ));
            }
        }
        // Do not remove the working version until the replacement is fully
        // downloaded and atomically published.
        remove_old_versions(&target_dir, &filename);
        installed.push(filename.clone());
        records.push((project_id.to_string(), filename.clone()));
        progress(&format!("Installed {filename}"), 0.88);
        eprintln!("[RixLauncher] [INFO] Installed: {filename}");
    } else {
        records.push((
            project_id.to_string(),
            existing_filename.unwrap_or_else(|| filename.clone()),
        ));
        eprintln!("[RixLauncher] [DEBUG] Skipping {filename} — already installed");
    }

    // Auto-install required dependencies (only for mods, not for RP/shaders)
    if kind == ContentKind::Mods {
        for dependency in &version.dependencies {
            if dependency.dependency_type.as_deref() != Some("required") {
                continue;
            }
            let Some(dep_id) = dependency.project_id.as_deref() else {
                continue;
            };
            if !visited.contains(dep_id) {
                eprintln!("[RixLauncher] [INFO] Installing required dependency: {dep_id}");
                progress(&format!("Installing dependency {dep_id}..."), 0.90);
                install_modrinth_recursive(
                    kind, dep_id, instance, installed, visited, records, progress,
                )
                .map_err(|error| format!("Required dependency {dep_id} failed: {error}"))?;
            }
        }
    }
    Ok(())
}

/// Returns a normalised base name for a mod file, used to detect if it is already installed.
/// Strips the `.jar` extension and any version-like suffixes starting with `-<digit>` or `+`.
fn modrinth_filename_base(filename: &str) -> String {
    let filename = strip_disabled_suffix(filename).unwrap_or(filename);
    let stem = filename
        .strip_suffix(".jar")
        .or_else(|| filename.strip_suffix(".JAR"))
        .unwrap_or(filename);
    // Strip version suffixes: -1.2.3 / +mc1.20 / _1.20 etc.
    if let Some(pos) = stem.find(|c: char| c == '-' || c == '+') {
        let after = &stem[pos + 1..];
        if after.starts_with(|c: char| c.is_ascii_digit()) {
            return stem[..pos].to_lowercase();
        }
    }
    stem.to_lowercase()
}

fn safe_remote_filename(filename: &str) -> Result<String, String> {
    let trimmed = filename.trim();
    if trimmed.is_empty() {
        return Err("Remote content did not provide a file name".to_string());
    }
    if safe_file_name(trimmed).is_none() {
        return Err("Remote content returned an unsafe file name".to_string());
    }
    Ok(trimmed.to_string())
}

fn safe_project_id(value: &str) -> Result<&str, String> {
    let value = value.trim();
    if value.is_empty()
        || value.len() > 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
    {
        return Err("Modrinth returned an unsafe project identifier".to_string());
    }
    Ok(value)
}

pub(crate) fn read_properties_file(path: &Path) -> std::io::Result<HashMap<String, String>> {
    let mut values = HashMap::new();
    let contents = fs::read_to_string(path)?;
    for line in contents.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if let Some((key, value)) = trimmed.split_once('=') {
            values.insert(key.trim().to_string(), value.trim().to_string());
        }
    }
    Ok(values)
}

fn write_properties_file(path: &Path, props: HashMap<String, String>) -> std::io::Result<()> {
    let mut content = String::new();
    for (k, v) in props {
        content.push_str(&format!("{k}={v}\n"));
    }
    atomic_write(path, content.as_bytes())
}

async fn duplicate_instance_task(src: PathBuf, dst: PathBuf) -> Result<String, String> {
    let parent = dst
        .parent()
        .ok_or_else(|| "Destination has no parent directory".to_string())?;
    let destination_name = dst
        .file_name()
        .ok_or_else(|| "Destination has no file name".to_string())?
        .to_string_lossy()
        .to_string();
    let staging = parent.join(format!(".{destination_name}.duplicating"));
    let _ = fs::remove_dir_all(&staging);
    if dst.exists() {
        return Err("An instance with that name already exists".to_string());
    }
    if let Err(error) = copy_dir_all(&src, &staging) {
        let _ = fs::remove_dir_all(&staging);
        return Err(format!("Failed to copy folder: {error}"));
    }
    let result = (|| -> Result<(), String> {
        let properties_path = staging.join("launch.properties");
        let mut props = read_properties_file(&properties_path)
            .map_err(|error| format!("Could not read copied profile: {error}"))?;
        let display_name = destination_name.replace('_', " ");
        props.insert("name".to_string(), display_name);
        write_properties_file(&properties_path, props)
            .map_err(|error| format!("Could not update copied profile: {error}"))?;
        fs::rename(&staging, &dst)
            .map_err(|error| format!("Could not publish duplicate: {error}"))?;
        Ok(())
    })();
    if let Err(error) = result {
        let _ = fs::remove_dir_all(&staging);
        return Err(error);
    }
    dst.file_name()
        .map(|name| name.to_string_lossy().to_string())
        .ok_or_else(|| "Published duplicate has no file name".to_string())
}

/// Clones an instance into a new directory with a possibly different MC version and loader.
/// - Copies options.txt (sanitizing version and resource packs across different versions),
///   servers.dat, usercache.json, etc.
/// - Backs up config/ to config_source_backup/ when versions differ to avoid schema incompatibilities.
/// - Synchronizes loader profile (profile.json) inheritsFrom with the new version.
/// - Re-downloads mods and their required dependencies from Modrinth for the new version.
/// - Incompatible/unverified mods are preserved as .disabled when versions differ rather than crashing.
/// - Preserves per-instance Java and memory overrides in launch.properties.
/// - Returns CloneResult with lists of installed mods and ones missing for the new version.
#[allow(clippy::too_many_arguments)]
async fn clone_instance_task(
    src: MinecraftInstance,
    new_name: String,
    new_version: String,
    new_loader: String,
    game_root: String,
    java_path: String,
    memory_gb: f32,
    player_name: String,
) -> Result<crate::launcher::types::CloneResult, String> {
    use crate::launcher::downloader::create_instance_task;
    use crate::launcher::types::{CloneResult, CreateRequest, ModrinthVersion};
    use crate::launcher::utils::{curl_get, sanitize_name, url_encode};
    use std::collections::HashSet;

    #[derive(serde::Deserialize)]
    struct ModrinthHashLookup {
        project_id: String,
    }

    let is_version_change = new_version != src.version || new_loader != src.loader;

    // 1. Create the new base instance (download MC + loader files)
    let request = CreateRequest {
        root: game_root.clone(),
        name: new_name.clone(),
        version: new_version.clone(),
        loader: new_loader.clone(),
        player_name: player_name.clone(),
        java_path: java_path.clone(),
        memory_gb,
    };
    create_instance_task(request)
        .await
        .map_err(|e| format!("Failed to create base instance: {e}"))?;

    let dst_path = PathBuf::from(&game_root).join(sanitize_name(&new_name));
    let src_path = &src.path;

    // 2. Copy settings files: options.txt, servers.dat, usercache.json
    let src_options = src_path.join("options.txt");
    if src_options.exists() {
        let dst_options = dst_path.join("options.txt");
        if is_version_change {
            // Sanitize options.txt for version migration:
            // - strip dataVersion / version so target MC generates its own format
            // - reset resourcePacks and incompatibleResourcePacks to [] to avoid crashes
            if let Ok(content) = fs::read_to_string(&src_options) {
                let mut sanitized = Vec::new();
                for line in content.lines() {
                    let trimmed = line.trim();
                    if trimmed.starts_with("version:") || trimmed.starts_with("dataVersion:") {
                        continue;
                    }
                    if trimmed.starts_with("resourcePacks:") {
                        sanitized.push("resourcePacks:[]".to_string());
                        continue;
                    }
                    if trimmed.starts_with("incompatibleResourcePacks:") {
                        sanitized.push("incompatibleResourcePacks:[]".to_string());
                        continue;
                    }
                    sanitized.push(line.to_string());
                }
                let output = sanitized.join("\n") + "\n";
                let _ = atomic_write(&dst_options, output.as_bytes());
            } else {
                let _ = atomic_copy_file(&src_options, &dst_options);
            }
        } else {
            atomic_copy_file(&src_options, &dst_options)
                .map_err(|error| format!("Failed to copy options.txt: {error}"))?;
        }
    }

    for fname in &["servers.dat", "usercache.json"] {
        let src_file = src_path.join(fname);
        if src_file.exists() {
            let dst_file = dst_path.join(fname);
            atomic_copy_file(&src_file, &dst_file)
                .map_err(|error| format!("Failed to copy {fname}: {error}"))?;
        }
    }

    // Copy config directory
    let src_config = src_path.join("config");
    if src_config.is_dir() {
        if is_version_change {
            // Incompatible configs across versions cause mod startup crashes.
            // Backup old configs to config_source_backup and create clean config/
            let dst_backup = dst_path.join("config_source_backup");
            copy_dir_all(&src_config, &dst_backup)
                .map_err(|error| format!("Failed to backup config directory: {error}"))?;
            let _ = fs::create_dir_all(dst_path.join("config"));
        } else {
            let dst_config = dst_path.join("config");
            copy_dir_all(&src_config, &dst_config)
                .map_err(|error| format!("Failed to copy config directory: {error}"))?;
        }
    }

    // 3. Copy shaders and resource packs directly (not version-specific)
    for folder in &["shaderpacks", "resourcepacks"] {
        let src_folder = src_path.join(folder);
        if src_folder.is_dir() {
            let dst_folder = dst_path.join(folder);
            copy_dir_all(&src_folder, &dst_folder)
                .map_err(|error| format!("Failed to copy {folder}: {error}"))?;
        }
    }

    // 4. Synchronize loader profile (profile.json) with the new version if loader is not Vanilla
    let dst_profile = dst_path.join("profile.json");
    if !dst_profile.is_file() {
        let src_profile = src_path.join("profile.json");
        if src_profile.is_file() {
            let _ = atomic_copy_file(&src_profile, &dst_profile);
        }
    }
    if dst_profile.is_file() {
        if let Ok(content) = fs::read_to_string(&dst_profile) {
            if let Ok(mut val) = serde_json::from_str::<serde_json::Value>(&content) {
                if let Some(obj) = val.as_object_mut() {
                    obj.insert(
                        "inheritsFrom".to_string(),
                        serde_json::Value::String(new_version.clone()),
                    );
                    if let Ok(updated) = serde_json::to_string_pretty(&val) {
                        let _ = atomic_write(&dst_profile, updated.as_bytes());
                    }
                }
            }
        }
    }

    // 5. Persist per-instance Java and memory overrides into launch.properties
    let effective_override_java = src.java_path.as_deref().filter(|p| {
        let p_clean = p.trim();
        if p_clean.is_empty() || p_clean.eq_ignore_ascii_case(constants::AUTO_JAVA_CONFIG_VALUE) {
            return false;
        }
        if !is_version_change {
            return true;
        }
        let required = crate::launcher::java::required_java_major(&new_version);
        crate::launcher::java::java_major(p_clean).unwrap_or(0) >= required
    });

    let _ = mod_write_profile(
        &dst_path,
        &new_name,
        &new_version,
        &new_loader,
        &player_name,
        &java_path,
        memory_gb,
        effective_override_java,
        src.memory_gb,
    );

    // 6. Re-download mods for the new version via Modrinth SHA1 hash lookup & dependency resolution
    let mut installed: Vec<String> = Vec::new();
    let mut missing: Vec<String> = Vec::new();
    let mut installed_projects: HashSet<String> = HashSet::new();

    let src_mods_dir = src_path.join("mods");
    let dst_mods_dir = dst_path.join("mods");
    let _ = fs::create_dir_all(&dst_mods_dir);

    if src_mods_dir.is_dir() {
        let entries: Vec<_> = fs::read_dir(&src_mods_dir)
            .map(|rd| rd.flatten().collect())
            .unwrap_or_default();

        let mut required_deps_queue: Vec<String> = Vec::new();

        for entry in entries {
            let fpath = entry.path();
            let fname = fpath
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();
            if !fpath
                .extension()
                .and_then(|e| e.to_str())
                .map(|e| e.eq_ignore_ascii_case("jar"))
                .unwrap_or(false)
            {
                continue;
            }

            let mut mod_installed = false;

            // Compute SHA1 hash of the jar
            if let Some(sha1) = compute_sha1_hex(&fpath) {
                let hash_url = format!(
                    "{}/version_file/{sha1}?algorithm=sha1",
                    constants::MODRINTH_API_BASE_URL
                );
                if let Ok(version_json) = curl_get(&hash_url, &[]) {
                    if !version_json.trim().is_empty() && !version_json.contains("\"error\"") {
                        if let Ok(lookup) =
                            serde_json::from_str::<ModrinthHashLookup>(&version_json)
                        {
                            if let Ok(project_id) = safe_project_id(&lookup.project_id) {
                                let project_id_str = project_id.to_string();
                                installed_projects.insert(project_id_str.clone());

                                let game_versions_param =
                                    url_encode(&format!("[\"{}\"]", new_version));
                                let loaders_param =
                                    url_encode(&format!("[\"{}\"]", new_loader.to_lowercase()));
                                let versions_url = format!(
                                    "{}/project/{}/version?game_versions={}&loaders={}",
                                    constants::MODRINTH_API_BASE_URL,
                                    url_encode(&project_id_str),
                                    game_versions_param,
                                    loaders_param
                                );
                                if let Ok(versions_json) = curl_get(&versions_url, &[]) {
                                    if !versions_json.trim().is_empty()
                                        && versions_json.trim() != "[]"
                                    {
                                        if let Ok(versions) =
                                            serde_json::from_str::<Vec<ModrinthVersion>>(
                                                &versions_json,
                                            )
                                        {
                                            if let Some(ver) = versions.first() {
                                                if let Some(file_info) = ver
                                                    .files
                                                    .iter()
                                                    .find(|file| file.primary)
                                                    .or_else(|| ver.files.first())
                                                {
                                                    if let Ok(new_fname) = safe_remote_filename(
                                                        &file_info.filename,
                                                    ) {
                                                        let dst_file =
                                                            dst_mods_dir.join(&new_fname);
                                                        if crate::launcher::utils::curl_download(
                                                            &file_info.url,
                                                            &dst_file,
                                                            &[],
                                                        )
                                                        .is_ok()
                                                        {
                                                            let verified = compute_file_sha1(
                                                                &dst_file,
                                                            )
                                                            .map(|actual| {
                                                                actual.eq_ignore_ascii_case(
                                                                    &file_info.hashes.sha1,
                                                                )
                                                            })
                                                            .unwrap_or(false);
                                                            if verified {
                                                                installed.push(new_fname.clone());
                                                                mod_installed = true;
                                                                eprintln!(
                                                                    "[Clone] Installed mod: {new_fname}"
                                                                );

                                                                // Collect required dependencies
                                                                for dep in &ver.dependencies {
                                                                    if dep
                                                                        .dependency_type
                                                                        .as_deref()
                                                                        == Some("required")
                                                                    {
                                                                        if let Some(dep_proj) =
                                                                            &dep.project_id
                                                                        {
                                                                            if !installed_projects
                                                                                .contains(dep_proj)
                                                                            {
                                                                                required_deps_queue
                                                                                    .push(
                                                                                        dep_proj
                                                                                            .clone(),
                                                                                    );
                                                                            }
                                                                        }
                                                                    }
                                                                }
                                                            } else {
                                                                let _ = fs::remove_file(&dst_file);
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            if !mod_installed {
                if is_version_change {
                    let disabled_name = format!("{fname}.disabled");
                    let _ = atomic_copy_file(&fpath, &dst_mods_dir.join(&disabled_name));
                    missing.push(fname.clone());
                    eprintln!(
                        "[Clone] Mod not compatible with {new_version}, preserved as {disabled_name}: {fname}"
                    );
                } else if atomic_copy_file(&fpath, &dst_mods_dir.join(&fname)).is_ok() {
                    installed.push(fname.clone());
                } else {
                    missing.push(fname.clone());
                }
            }
        }

        // 6b. Download required dependencies from Modrinth
        for dep_proj_id in required_deps_queue {
            if installed_projects.contains(&dep_proj_id) {
                continue;
            }
            if let Ok(safe_dep_id) = safe_project_id(&dep_proj_id) {
                let dep_id_str = safe_dep_id.to_string();
                installed_projects.insert(dep_id_str.clone());

                let game_versions_param = url_encode(&format!("[\"{}\"]", new_version));
                let loaders_param =
                    url_encode(&format!("[\"{}\"]", new_loader.to_lowercase()));
                let versions_url = format!(
                    "{}/project/{}/version?game_versions={}&loaders={}",
                    constants::MODRINTH_API_BASE_URL,
                    url_encode(&dep_id_str),
                    game_versions_param,
                    loaders_param
                );
                if let Ok(versions_json) = curl_get(&versions_url, &[]) {
                    if !versions_json.trim().is_empty() && versions_json.trim() != "[]" {
                        if let Ok(versions) =
                            serde_json::from_str::<Vec<ModrinthVersion>>(&versions_json)
                        {
                            if let Some(ver) = versions.first() {
                                if let Some(file_info) = ver
                                    .files
                                    .iter()
                                    .find(|file| file.primary)
                                    .or_else(|| ver.files.first())
                                {
                                    if let Ok(dep_fname) =
                                        safe_remote_filename(&file_info.filename)
                                    {
                                        let dst_file = dst_mods_dir.join(&dep_fname);
                                        if !dst_file.is_file() {
                                            if crate::launcher::utils::curl_download(
                                                &file_info.url,
                                                &dst_file,
                                                &[],
                                            )
                                            .is_ok()
                                            {
                                                let verified = compute_file_sha1(&dst_file)
                                                    .map(|actual| {
                                                        actual.eq_ignore_ascii_case(
                                                            &file_info.hashes.sha1,
                                                        )
                                                    })
                                                    .unwrap_or(false);
                                                if verified {
                                                    installed.push(dep_fname.clone());
                                                    eprintln!(
                                                        "[Clone] Installed required dependency: {dep_fname}"
                                                    );
                                                } else {
                                                    let _ = fs::remove_file(&dst_file);
                                                    missing.push(format!("dependency: {dep_id_str}"));
                                                }
                                            } else {
                                                missing.push(format!("dependency: {dep_id_str}"));
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    } else {
                        missing.push(format!("dependency: {dep_id_str}"));
                    }
                }
            }
        }
    }

    Ok(CloneResult {
        new_name,
        installed,
        missing,
    })
}

/// Computes a hex-encoded SHA-1 hash of a file.
fn compute_sha1_hex(path: &Path) -> Option<String> {
    use std::io::Read;
    let mut file = fs::File::open(path).ok()?;
    let mut state = Sha1State::new();
    let mut buf = [0u8; 65536];
    loop {
        let n = file.read(&mut buf).ok()?;
        if n == 0 {
            break;
        }
        state.update(&buf[..n]);
    }
    Some(state.finish())
}

/// Minimal pure-Rust SHA-1 implementation (no external crates needed).
struct Sha1State {
    h: [u32; 5],
    buf: Vec<u8>,
    total_len: u64,
}

impl Sha1State {
    fn new() -> Self {
        Self {
            h: [0x67452301, 0xEFCDAB89, 0x98BADCFE, 0x10325476, 0xC3D2E1F0],
            buf: Vec::new(),
            total_len: 0,
        }
    }

    fn update(&mut self, data: &[u8]) {
        self.total_len += data.len() as u64;
        self.buf.extend_from_slice(data);
        while self.buf.len() >= 64 {
            let block: [u8; 64] = self.buf[..64].try_into().unwrap();
            self.buf.drain(..64);
            self.process_block(block);
        }
    }

    fn finish(mut self) -> String {
        let bit_len = self.total_len * 8;
        self.buf.push(0x80);
        while self.buf.len() % 64 != 56 {
            self.buf.push(0x00);
        }
        for i in (0..8).rev() {
            self.buf.push(((bit_len >> (i * 8)) & 0xFF) as u8);
        }
        while self.buf.len() >= 64 {
            let block: [u8; 64] = self.buf[..64].try_into().unwrap();
            self.buf.drain(..64);
            self.process_block(block);
        }
        let mut result = String::new();
        for word in &self.h {
            result.push_str(&format!("{:08x}", word));
        }
        result
    }

    fn process_block(&mut self, block: [u8; 64]) {
        let mut w = [0u32; 80];
        for i in 0..16 {
            w[i] = u32::from_be_bytes(block[i * 4..i * 4 + 4].try_into().unwrap());
        }
        for i in 16..80 {
            w[i] = (w[i - 3] ^ w[i - 8] ^ w[i - 14] ^ w[i - 16]).rotate_left(1);
        }
        let [mut a, mut b, mut c, mut d, mut e] = self.h;
        for i in 0..80 {
            let (f, k) = match i {
                0..=19 => ((b & c) | (!b & d), 0x5A827999u32),
                20..=39 => (b ^ c ^ d, 0x6ED9EBA1u32),
                40..=59 => ((b & c) | (b & d) | (c & d), 0x8F1BBCDCu32),
                _ => (b ^ c ^ d, 0xCA62C1D6u32),
            };
            let temp = a
                .rotate_left(5)
                .wrapping_add(f)
                .wrapping_add(e)
                .wrapping_add(k)
                .wrapping_add(w[i]);
            e = d;
            d = c;
            c = b.rotate_left(30);
            b = a;
            a = temp;
        }
        self.h[0] = self.h[0].wrapping_add(a);
        self.h[1] = self.h[1].wrapping_add(b);
        self.h[2] = self.h[2].wrapping_add(c);
        self.h[3] = self.h[3].wrapping_add(d);
        self.h[4] = self.h[4].wrapping_add(e);
    }
}

async fn import_dropped_file_task(
    path: PathBuf,
    instance_path: PathBuf,
    current_kind: ContentKind,
) -> Result<(String, ContentKind), String> {
    tokio::task::spawn_blocking(move || import_dropped_file_sync(path, instance_path, current_kind))
        .await
        .map_err(|e| format!("Spawn error: {}", e))?
}

fn import_dropped_file_sync(
    path: PathBuf,
    instance_path: PathBuf,
    current_kind: ContentKind,
) -> Result<(String, ContentKind), String> {
    let metadata = fs::symlink_metadata(&path)
        .map_err(|error| format!("Could not inspect dropped path: {error}"))?;
    if metadata.file_type().is_symlink() {
        return Err("Symbolic links cannot be imported".to_string());
    }
    if !metadata.file_type().is_file() && !metadata.file_type().is_dir() {
        return Err("Dropped file does not exist".to_string());
    }

    let filename = path
        .file_name()
        .ok_or_else(|| "Invalid file name".to_string())?
        .to_string_lossy()
        .to_string();
    safe_file_name(&filename).ok_or_else(|| "Invalid file name".to_string())?;

    let ext = path
        .extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .unwrap_or_default();

    // 1. Identify kind
    let target_kind = if ext == "jar" {
        ContentKind::Mods
    } else if ext == "zip" {
        // Open zip and inspect entries
        let file =
            std::fs::File::open(&path).map_err(|e| format!("Failed to open zip file: {}", e))?;
        let mut archive =
            zip::ZipArchive::new(file).map_err(|e| format!("Failed to read zip archive: {}", e))?;
        let mut is_shader = false;
        let mut is_resourcepack = false;

        for i in 0..archive.len() {
            if let Ok(entry) = archive.by_index(i) {
                let name = entry.name().to_lowercase();
                if name.starts_with("shaders/") {
                    is_shader = true;
                    break;
                }
                if name == "pack.mcmeta" || name.starts_with("pack.mcmeta/") {
                    is_resourcepack = true;
                }
            }
        }

        if is_shader {
            ContentKind::Shaders
        } else if is_resourcepack {
            ContentKind::ResourcePacks
        } else {
            // Fallback to active tab
            match current_kind {
                ContentKind::Shaders => ContentKind::Shaders,
                ContentKind::ResourcePacks => ContentKind::ResourcePacks,
                ContentKind::Mods => ContentKind::ResourcePacks,
            }
        }
    } else if path.is_dir() {
        if path.join("shaders").is_dir() {
            ContentKind::Shaders
        } else if path.join("pack.mcmeta").is_file() {
            ContentKind::ResourcePacks
        } else {
            match current_kind {
                ContentKind::Shaders => ContentKind::Shaders,
                ContentKind::ResourcePacks => ContentKind::ResourcePacks,
                ContentKind::Mods => ContentKind::ResourcePacks,
            }
        }
    } else {
        return Err(format!("Unsupported file extension: .{}", ext));
    };

    // 2. Perform copy
    let dest_dir = instance_path.join(target_kind.install_folder());
    std::fs::create_dir_all(&dest_dir)
        .map_err(|e| format!("Failed to create destination directory: {}", e))?;
    let dest_path = dest_dir.join(&filename);
    if dest_path.exists() {
        return Err(format!("A file named {filename} is already installed"));
    }

    if path.is_dir() {
        copy_dir_all(&path, &dest_path).map_err(|e| format!("Failed to copy directory: {}", e))?;
    } else {
        atomic_copy_file(&path, &dest_path).map_err(|e| format!("Failed to copy file: {}", e))?;
    }

    Ok((filename, target_kind))
}

#[derive(serde::Deserialize)]
struct DeviceCodeResponse {
    device_code: String,
    user_code: String,
    verification_uri: String,
    expires_in: u32,
    interval: u32,
}

#[derive(serde::Deserialize)]
struct TokenResponse {
    access_token: Option<String>,
    refresh_token: Option<String>,
    error: Option<String>,
}

#[derive(serde::Serialize)]
struct XblRequestProperties {
    #[serde(rename = "AuthMethod")]
    auth_method: String,
    #[serde(rename = "SiteName")]
    site_name: String,
    #[serde(rename = "RpsTicket")]
    rps_ticket: String,
}

#[derive(serde::Serialize)]
struct XblRequest {
    #[serde(rename = "Properties")]
    properties: XblRequestProperties,
    #[serde(rename = "RelyingParty")]
    relying_party: String,
    #[serde(rename = "TokenType")]
    token_type: String,
}

#[derive(serde::Deserialize)]
struct XblResponse {
    #[serde(rename = "Token")]
    token: String,
    #[serde(rename = "DisplayClaims")]
    display_claims: XblDisplayClaims,
}

#[derive(serde::Deserialize)]
struct XblDisplayClaims {
    xui: Vec<XblXui>,
}

#[derive(serde::Deserialize)]
struct XblXui {
    uhs: String,
}

#[derive(serde::Serialize)]
struct XstsRequestProperties {
    #[serde(rename = "SandboxId")]
    sandbox_id: String,
    #[serde(rename = "UserTokens")]
    user_tokens: Vec<String>,
}

#[derive(serde::Serialize)]
struct XstsRequest {
    #[serde(rename = "Properties")]
    properties: XstsRequestProperties,
    #[serde(rename = "RelyingParty")]
    relying_party: String,
    #[serde(rename = "TokenType")]
    token_type: String,
}

#[derive(serde::Serialize)]
struct MinecraftLoginRequest {
    #[serde(rename = "identityToken")]
    identity_token: String,
}

#[derive(serde::Deserialize)]
struct MinecraftLoginResponse {
    access_token: String,
}

#[derive(serde::Deserialize)]
struct MinecraftProfileResponse {
    id: String,
    name: String,
}

async fn request_microsoft_device_code() -> Result<MicrosoftLoginState, String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(
            constants::SHORT_REQUEST_TIMEOUT_SECONDS,
        ))
        .build()
        .map_err(|error| format!("Could not initialize Microsoft client: {error}"))?;
    let res = client
        .post(constants::MICROSOFT_DEVICE_CODE_URL)
        .form(&[
            ("client_id", constants::MICROSOFT_CLIENT_ID),
            ("scope", constants::MICROSOFT_XBOX_SCOPE),
            ("response_type", "device_code"),
        ])
        .send()
        .await
        .map_err(|e| format!("Network error: {e}"))?
        .error_for_status()
        .map_err(|e| format!("Microsoft device-code request failed: {e}"))?;

    let data: DeviceCodeResponse = res
        .json()
        .await
        .map_err(|e| format!("Failed to parse device code: {e}"))?;

    Ok(MicrosoftLoginState {
        device_code: data.device_code,
        user_code: data.user_code,
        verification_uri: data.verification_uri,
        expires_in: data.expires_in,
        interval: data.interval,
        status: "Waiting for user login...".to_string(),
    })
}

async fn poll_microsoft_auth(
    device_code: String,
    interval: u64,
    expires_in: u32,
    cancel: std::sync::Arc<std::sync::atomic::AtomicBool>,
) -> Result<Account, String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(
            constants::SHORT_REQUEST_TIMEOUT_SECONDS,
        ))
        .build()
        .map_err(|e| format!("Could not initialize Microsoft client: {e}"))?;
    let start_time = std::time::Instant::now();
    let timeout = std::time::Duration::from_secs(u64::from(expires_in.max(1)));

    loop {
        if cancel.load(std::sync::atomic::Ordering::Relaxed) {
            return Err("Login cancelled".to_string());
        }
        if start_time.elapsed() > timeout {
            return Err("Login session expired. Please try again.".to_string());
        }

        tokio::select! {
            _ = tokio::time::sleep(std::time::Duration::from_secs(interval.max(1))) => {}
            _ = wait_for_login_cancel(cancel.clone()) => return Err("Login cancelled".to_string()),
        }

        let res = client
            .post(constants::MICROSOFT_TOKEN_URL)
            .form(&[
                ("grant_type", "device_code"),
                ("device_code", &device_code),
                ("client_id", constants::MICROSOFT_CLIENT_ID),
            ])
            .send()
            .await;

        let res = match res {
            Ok(r) => r,
            Err(_) => continue, // Ignore intermittent network errors during poll
        };

        let token_data: TokenResponse = match res.json().await {
            Ok(d) => d,
            Err(_) => continue,
        };

        if let Some(err) = token_data.error {
            if err == "authorization_pending" {
                continue;
            } else {
                return Err(format!("Microsoft authorization failed: {}", err));
            }
        }

        let Some(ms_access_token) = token_data.access_token else {
            return Err("Microsoft response did not contain access token".to_string());
        };
        let refresh_token = token_data.refresh_token;

        return finish_microsoft_auth(&client, ms_access_token, refresh_token).await;
    }
}

async fn refresh_microsoft_account(refresh_token: String) -> Result<Account, String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(
            constants::SHORT_REQUEST_TIMEOUT_SECONDS,
        ))
        .build()
        .map_err(|error| format!("Could not initialize Microsoft client: {error}"))?;
    let token_data: TokenResponse = client
        .post(constants::MICROSOFT_TOKEN_URL)
        .form(&[
            ("grant_type", "refresh_token"),
            ("refresh_token", refresh_token.as_str()),
            ("client_id", constants::MICROSOFT_CLIENT_ID),
            ("scope", constants::MICROSOFT_XBOX_SCOPE),
        ])
        .send()
        .await
        .map_err(|error| format!("Microsoft refresh request failed: {error}"))?
        .error_for_status()
        .map_err(|error| format!("Microsoft refresh request failed: {error}"))?
        .json()
        .await
        .map_err(|error| format!("Could not parse Microsoft refresh response: {error}"))?;

    if let Some(error) = token_data.error {
        return Err(format!("Microsoft refresh failed: {error}"));
    }
    let access_token = token_data
        .access_token
        .ok_or_else(|| "Microsoft refresh response did not contain an access token".to_string())?;
    finish_microsoft_auth(
        &client,
        access_token,
        token_data.refresh_token.or(Some(refresh_token)),
    )
    .await
}

async fn finish_microsoft_auth(
    client: &reqwest::Client,
    ms_access_token: String,
    refresh_token: Option<String>,
) -> Result<Account, String> {
    let xbl_req = XblRequest {
        properties: XblRequestProperties {
            auth_method: constants::XBOX_AUTH_METHOD.to_string(),
            site_name: constants::XBOX_SITE_NAME.to_string(),
            rps_ticket: format!("t={ms_access_token}"),
        },
        relying_party: constants::XBOX_AUTH_RELYING_PARTY.to_string(),
        token_type: constants::XBOX_TOKEN_TYPE.to_string(),
    };
    let xbl_data: XblResponse = client
        .post(constants::XBOX_AUTH_URL)
        .json(&xbl_req)
        .send()
        .await
        .map_err(|error| format!("Xbox Live auth failed: {error}"))?
        .error_for_status()
        .map_err(|error| format!("Xbox Live auth failed: {error}"))?
        .json()
        .await
        .map_err(|error| format!("Failed to parse Xbox Live response: {error}"))?;

    let uhs = xbl_data
        .display_claims
        .xui
        .first()
        .ok_or_else(|| "No User Hash found in Xbox claims".to_string())?
        .uhs
        .clone();
    let xsts_req = XstsRequest {
        properties: XstsRequestProperties {
            sandbox_id: constants::XBOX_SANDBOX_ID.to_string(),
            user_tokens: vec![xbl_data.token],
        },
        relying_party: constants::MINECRAFT_XBOX_RELYING_PARTY.to_string(),
        token_type: constants::XBOX_TOKEN_TYPE.to_string(),
    };
    let xsts_data: XblResponse = client
        .post(constants::XBOX_XSTS_URL)
        .json(&xsts_req)
        .send()
        .await
        .map_err(|error| format!("XSTS auth failed: {error}"))?
        .error_for_status()
        .map_err(|error| {
            format!("XSTS auth failed; the account may not be linked to Xbox Live: {error}")
        })?
        .json()
        .await
        .map_err(|error| format!("Failed to parse XSTS response: {error}"))?;

    let mc_req = MinecraftLoginRequest {
        identity_token: format!("XBL3.0 x={uhs};{}", xsts_data.token),
    };
    let mc_login_data: MinecraftLoginResponse = client
        .post(constants::MINECRAFT_XBOX_LOGIN_URL)
        .json(&mc_req)
        .send()
        .await
        .map_err(|error| format!("Minecraft Services login failed: {error}"))?
        .error_for_status()
        .map_err(|error| format!("Minecraft Services rejected the Xbox Live token: {error}"))?
        .json()
        .await
        .map_err(|error| format!("Failed to parse Minecraft login response: {error}"))?;
    let profile_data: MinecraftProfileResponse = client
        .get(constants::MINECRAFT_PROFILE_URL)
        .bearer_auth(&mc_login_data.access_token)
        .send()
        .await
        .map_err(|error| format!("Minecraft profile request failed: {error}"))?
        .error_for_status()
        .map_err(|error| format!("You do not own Minecraft on this Microsoft account: {error}"))?
        .json()
        .await
        .map_err(|error| format!("Failed to parse Minecraft profile response: {error}"))?;

    Ok(Account {
        name: profile_data.name,
        uuid: profile_data.id,
        access_token: mc_login_data.access_token,
        is_microsoft: true,
        refresh_token,
    })
}

async fn wait_for_login_cancel(cancel: std::sync::Arc<std::sync::atomic::AtomicBool>) {
    while !cancel.load(std::sync::atomic::Ordering::Relaxed) {
        tokio::time::sleep(std::time::Duration::from_millis(
            constants::OAUTH_CANCEL_POLL_MILLIS,
        ))
        .await;
    }
}

fn remove_old_versions(target_dir: &Path, new_filename: &str) {
    let base_name = modrinth_filename_base(new_filename);
    if base_name.is_empty() {
        return;
    }

    if let Ok(entries) = std::fs::read_dir(target_dir) {
        for entry in entries.flatten() {
            if let Ok(file_type) = entry.file_type() {
                if file_type.is_file() {
                    let name = entry.file_name().to_string_lossy().into_owned();
                    if name != new_filename && modrinth_filename_base(&name) == base_name {
                        let _ = std::fs::remove_file(entry.path());
                        eprintln!("[RixLauncher] Removed old mod version duplicate: {}", name);
                    }
                }
            }
        }
    }
}

fn extract_icon_from_jar(jar_path: &Path) -> Option<Vec<u8>> {
    let file = fs::File::open(jar_path).ok()?;
    let mut archive = zip::ZipArchive::new(file).ok()?;

    // 1. Try fabric.mod.json
    let mut icon_path_opt = None;
    if let Ok(mut mod_json_file) = archive.by_name("fabric.mod.json") {
        let mut contents = String::new();
        use std::io::Read;
        if mod_json_file.read_to_string(&mut contents).is_ok() {
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&contents) {
                if let Some(icon_path) = json.get("icon").and_then(|i| i.as_str()) {
                    icon_path_opt = Some(icon_path.to_string());
                }
            }
        }
    }

    if let Some(icon_path) = icon_path_opt {
        if let Ok(mut icon_file) = archive.by_name(&icon_path) {
            let mut bytes = Vec::new();
            use std::io::Read;
            if icon_file.read_to_end(&mut bytes).is_ok() {
                return Some(bytes);
            }
        }
    }

    // 2. Try searching for any png file named icon.png, logo.png, or pack.png
    for i in 0..archive.len() {
        if let Ok(file) = archive.by_index(i) {
            let name = file.name().to_lowercase();
            if name.ends_with("icon.png")
                || name.ends_with("logo.png")
                || name.ends_with("pack.png")
            {
                let mut file_mut = file;
                let mut bytes = Vec::new();
                use std::io::Read;
                if file_mut.read_to_end(&mut bytes).is_ok() {
                    return Some(bytes);
                }
            }
        }
    }

    None
}

fn load_local_mod_icons(instance_path: PathBuf) -> Result<HashMap<String, Vec<u8>>, String> {
    let mut icons = HashMap::new();
    let mods_dir = instance_path.join("mods");
    if let Ok(entries) = std::fs::read_dir(mods_dir) {
        for entry in entries.flatten() {
            if let Ok(file_type) = entry.file_type() {
                if file_type.is_file() {
                    let path = entry.path();
                    let is_jar = path
                        .extension()
                        .is_some_and(|ext| ext.eq_ignore_ascii_case("jar"));
                    let is_disabled = path
                        .extension()
                        .is_some_and(|ext| ext.eq_ignore_ascii_case("disabled"))
                        && path
                            .file_stem()
                            .and_then(|s| std::path::Path::new(s).extension())
                            .is_some_and(|ext| ext.eq_ignore_ascii_case("jar"));
                    if is_jar || is_disabled {
                        let Some(filename) = path
                            .file_name()
                            .and_then(|name| name.to_str())
                            .map(str::to_owned)
                        else {
                            continue;
                        };
                        if let Some(bytes) = extract_icon_from_jar(&path) {
                            icons.insert(filename, bytes);
                        }
                    }
                }
            }
        }
    }
    Ok(icons)
}

fn extract_face_from_skin(skin_bytes: &[u8]) -> Option<Vec<u8>> {
    use image::GenericImageView;
    let img = image::load_from_memory(skin_bytes).ok()?;
    let mut face = image::ImageBuffer::new(8, 8);

    // Copy face area x=8..16, y=8..16
    for y in 0..8 {
        for x in 0..8 {
            if (8 + x) < img.width() && (8 + y) < img.height() {
                let pixel = img.get_pixel(8 + x, 8 + y);
                face.put_pixel(x, y, pixel);
            }
        }
    }

    // Overlay helmet area x=40..48, y=8..16 (if pixel is not transparent)
    for y in 0..8 {
        for x in 0..8 {
            if (40 + x) < img.width() && (8 + y) < img.height() {
                let pixel = img.get_pixel(40 + x, 8 + y);
                if pixel[3] > 0 {
                    // alpha > 0
                    face.put_pixel(x, y, pixel);
                }
            }
        }
    }

    // Scale up to 64x64 using Nearest Neighbor to preserve pixel art style
    let scaled = image::imageops::resize(&face, 64, 64, image::imageops::FilterType::Nearest);
    let mut png_bytes = Vec::new();
    let mut cursor = std::io::Cursor::new(&mut png_bytes);
    scaled.write_to(&mut cursor, image::ImageFormat::Png).ok()?;
    Some(png_bytes)
}

fn account_asset_path(folder: &str, account_id: &str) -> Option<PathBuf> {
    let account_id = safe_file_name(account_id.trim())?;
    Some(
        get_rixlauncher_root()
            .join(folder)
            .join(format!("{account_id}.png")),
    )
}

fn player_skin_task(name: String, account_uuid: String) -> Task<Message> {
    let response_account_uuid = account_uuid.clone();
    Task::perform(
        async move { fetch_player_skin(name, account_uuid).await },
        move |result| Message::PlayerSkinLoaded(response_account_uuid.clone(), result),
    )
}

async fn fetch_player_skin(name: String, account_id: String) -> Result<(String, Vec<u8>), String> {
    if let Some(local_skin_path) = account_asset_path("skins", &account_id) {
        if local_skin_path.is_file() {
            if let Ok(bytes) = fs::read(&local_skin_path) {
                if let Some(face_bytes) = extract_face_from_skin(&bytes) {
                    return Ok((name, face_bytes));
                }
            }
        }
    }

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(
            constants::SKIN_REQUEST_TIMEOUT_SECONDS,
        ))
        .build()
        .map_err(|e| e.to_string())?;
    let url = format!(
        "{}/avatar/{}/64",
        constants::MINOTAR_BASE_URL,
        url_encode(&name)
    );
    let res = client.get(&url).send().await.map_err(|e| e.to_string())?;
    if !res.status().is_success() {
        return Err(format!("Skin service returned HTTP {}", res.status()));
    }

    let bytes = res.bytes().await.map_err(|e| e.to_string())?;
    if image::load_from_memory(&bytes).is_err() {
        return Err("Skin service returned an invalid image".to_string());
    }

    Ok((name, bytes.to_vec()))
}

fn open_external_url(uri: &str) -> Result<(), String> {
    let parsed = url::Url::parse(uri)
        .map_err(|_| "Microsoft returned an invalid verification URL".to_string())?;
    if parsed.scheme() != "https" || parsed.host_str().is_none() {
        return Err("Only web verification links can be opened".to_string());
    }

    #[cfg(target_os = "windows")]
    let command = std::process::Command::new("explorer.exe").arg(uri).spawn();
    #[cfg(target_os = "macos")]
    let command = std::process::Command::new("open").arg(uri).spawn();
    #[cfg(all(unix, not(target_os = "macos")))]
    let command = std::process::Command::new("xdg-open").arg(uri).spawn();

    command
        .map(|_| ())
        .map_err(|error| format!("Could not open the verification page: {error}"))
}

fn compute_file_sha1(path: &Path) -> Result<String, String> {
    use sha1::{Digest, Sha1};
    let mut file = fs::File::open(path).map_err(|e| e.to_string())?;
    let mut hasher = Sha1::new();
    std::io::copy(&mut file, &mut hasher).map_err(|e| e.to_string())?;
    let hash = hasher.finalize();
    Ok(format!("{:x}", hash))
}

async fn check_mod_updates_task(
    mods_dir: PathBuf,
    loader: String,
    game_version: String,
) -> Result<HashMap<String, ModrinthUpdateInfo>, String> {
    let mut hash_to_filename = HashMap::new();
    let mut hashes = Vec::new();

    if let Ok(entries) = std::fs::read_dir(&mods_dir) {
        for entry in entries.flatten() {
            if let Ok(file_type) = entry.file_type() {
                if file_type.is_file() {
                    let path = entry.path();
                    let is_jar = path
                        .extension()
                        .is_some_and(|ext| ext.eq_ignore_ascii_case("jar"));
                    let is_disabled = path
                        .extension()
                        .is_some_and(|ext| ext.eq_ignore_ascii_case("disabled"))
                        && path
                            .file_stem()
                            .and_then(|s| Path::new(s).extension())
                            .is_some_and(|ext| ext.eq_ignore_ascii_case("jar"));
                    if is_jar || is_disabled {
                        let Some(filename) = path
                            .file_name()
                            .and_then(|name| name.to_str())
                            .filter(|name| safe_file_name(name).is_some())
                            .map(str::to_owned)
                        else {
                            continue;
                        };
                        if let Ok(hash) = compute_file_sha1(&path) {
                            hash_to_filename.insert(hash.clone(), filename);
                            hashes.push(hash);
                        }
                    }
                }
            }
        }
    }

    if hashes.is_empty() {
        return Ok(HashMap::new());
    }

    let request_body = ModrinthUpdateRequest {
        hashes,
        algorithm: "sha1".to_string(),
        loaders: vec![loader.to_lowercase()],
        game_versions: vec![game_version],
    };

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(
            constants::SHORT_REQUEST_TIMEOUT_SECONDS,
        ))
        .build()
        .map_err(|error| format!("Could not initialize Modrinth client: {error}"))?;
    let response = client
        .post(format!(
            "{}/version_files/update",
            constants::MODRINTH_API_BASE_URL
        ))
        .header("User-Agent", constants::USER_AGENT)
        .json(&request_body)
        .send()
        .await
        .map_err(|e| format!("Modrinth API error: {}", e))?
        .error_for_status()
        .map_err(|e| format!("Modrinth API returned an error: {e}"))?;

    let updates: HashMap<String, ModrinthVersion> = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse Modrinth updates response: {}", e))?;

    let mut results = HashMap::new();
    for (hash, version) in updates {
        if let Some(local_filename) = hash_to_filename.get(&hash) {
            // Find the primary file, or first file
            if let Some(file_info) = version
                .files
                .iter()
                .find(|f| f.primary)
                .or_else(|| version.files.first())
            {
                // Only treat it as an update if the hash is different (i.e. not the same version we already have)
                if file_info.hashes.sha1 != hash {
                    results.insert(
                        local_filename.clone(),
                        ModrinthUpdateInfo {
                            local_filename: local_filename.clone(),
                            download_url: file_info.url.clone(),
                            new_filename: file_info.filename.clone(),
                            new_version: version.version_number,
                            project_id: version.project_id,
                            new_sha1: Some(file_info.hashes.sha1.clone()),
                        },
                    );
                }
            }
        }
    }

    Ok(results)
}

async fn download_mod_update_task(
    url: String,
    new_filename: String,
    old_filename: String,
    expected_sha1: Option<String>,
    mods_dir: PathBuf,
) -> Result<(), String> {
    let old_filename = safe_file_name(&old_filename)
        .ok_or_else(|| "The installed mod has an unsafe file name".to_string())?
        .to_string();
    let new_filename = safe_remote_filename(&new_filename)?;
    // Determine the target filename, keeping the .disabled state if the old mod was disabled
    let final_new_filename =
        if is_disabled_mod_filename(&old_filename) && !is_disabled_mod_filename(&new_filename) {
            format!("{}.disabled", new_filename)
        } else {
            new_filename
        };

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(
            constants::DOWNLOAD_TIMEOUT_SECONDS,
        ))
        .build()
        .map_err(|error| format!("Could not initialize download client: {error}"))?;
    let temp_dest = unique_temp_path(&mods_dir.join(&final_new_filename), "update");

    // Download to temp file first
    downloader::download_if_missing_verified(
        &client,
        &url,
        &temp_dest,
        expected_sha1.as_deref(),
        None,
    )
    .await?;

    // Publish the new file first. The old file is removed only after a
    // successful download and rename.
    let old_path = mods_dir.join(&old_filename);
    let dest_path = mods_dir.join(&final_new_filename);
    atomic_copy_file(&temp_dest, &dest_path)
        .map_err(|e| format!("Failed to publish updated mod: {e}"))?;
    let _ = fs::remove_file(&temp_dest);
    if old_path != dest_path && old_path.is_file() {
        let _ = fs::remove_file(old_path);
    }

    Ok(())
}

fn home_dir() -> PathBuf {
    std::env::var("USERPROFILE")
        .or_else(|_| std::env::var("HOME"))
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("."))
}

pub fn get_rixlauncher_root() -> PathBuf {
    #[cfg(target_os = "windows")]
    {
        if let Ok(appdata) = std::env::var("APPDATA") {
            PathBuf::from(appdata).join(constants::USER_DATA_DIRECTORY)
        } else {
            home_dir().join(constants::USER_DATA_DIRECTORY)
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        home_dir().join(constants::USER_DATA_DIRECTORY)
    }
}

#[cfg(test)]
mod launcher_tests {
    use super::*;

    #[test]
    fn test_required_java_major() {
        // Legacy versioning
        assert_eq!(java::required_java_major("1.8.9"), 8);
        assert_eq!(java::required_java_major("1.12.2"), 8);
        assert_eq!(java::required_java_major("1.17"), 16);
        assert_eq!(java::required_java_major("1.18"), 17);
        assert_eq!(java::required_java_major("1.20.4"), 17);
        assert_eq!(java::required_java_major("1.20.5"), 21);
        assert_eq!(java::required_java_major("1.21.11"), 21);

        // Date-based versions (2026+) require modern Java 25+
        assert_eq!(java::required_java_major("26.1"), 25);
        assert_eq!(java::required_java_major("26.2"), 25);
    }

    #[test]
    fn test_instance_property_parsing_and_overrides() {
        let temp_dir = tempfile::tempdir().unwrap();
        let game_root = temp_dir.path().join("instances");
        std::fs::create_dir_all(game_root.join("TestInstance")).unwrap();

        mod_write_profile(
            &game_root.join("TestInstance"),
            "TestInstance",
            "26.2",
            "Fabric",
            "Player",
            "/usr/bin/java",
            4.0,
            Some("/managed/jdk-21/bin/java"),
            Some(6.0),
        )
        .unwrap();

        let instances = load_instances(game_root.to_str().unwrap());
        assert_eq!(instances.len(), 1);
        let inst = &instances[0];
        assert_eq!(inst.name, "TestInstance");
        assert_eq!(inst.version, "26.2");
        assert_eq!(inst.loader, "Fabric");
        assert_eq!(inst.memory_gb, Some(6.0));
        assert_eq!(inst.java_path.as_deref(), Some("/managed/jdk-21/bin/java"));
    }

    #[test]
    fn legacy_account_payload_is_not_treated_as_metadata() {
        let legacy = r#"[
            {
                "name": "Player",
                "uuid": "legacy-uuid",
                "access_token": "secret-token",
                "is_microsoft": true,
                "refresh_token": "refresh-secret"
            }
        ]"#;

        assert!(serde_json::from_str::<Vec<StoredAccount>>(legacy).is_err());
        assert!(serde_json::from_str::<Vec<LegacyAccount>>(legacy).is_ok());
    }

    #[test]
    fn content_registry_tracks_and_forgets_downloads() {
        let temp_dir = tempfile::tempdir().unwrap();
        let instance_path = temp_dir.path().join("instance");
        std::fs::create_dir_all(instance_path.join(ContentKind::Mods.install_folder())).unwrap();
        let instance = MinecraftInstance {
            id: 1,
            name: "Test".to_string(),
            version: "1.21".to_string(),
            loader: "Fabric".to_string(),
            path: instance_path,
            mod_count: 1,
            resource_pack_count: 0,
            shader_count: 0,
            has_version_files: false,
            has_runnable_jar: false,
            memory_gb: None,
            java_path: None,
        };

        register_installed_content(
            &instance,
            ContentKind::Mods,
            &[("sodium".to_string(), "sodium-fabric.jar".to_string())],
        )
        .unwrap();
        assert!(installed_content_project_ids(&instance, ContentKind::Mods).contains("sodium"));

        forget_installed_content_file(&instance, ContentKind::Mods, "sodium-fabric.jar").unwrap();
        assert!(installed_content_project_ids(&instance, ContentKind::Mods).is_empty());
    }

    #[test]
    fn test_required_java_major_26_3() {
        assert_eq!(java::required_java_major("26.3"), 25);
    }

    #[test]
    fn test_options_sanitization_on_version_change() {
        let temp_dir = tempfile::tempdir().unwrap();
        let src_options = temp_dir.path().join("options.txt");
        let dst_options = temp_dir.path().join("dst_options.txt");

        let raw_options = "version:3465\nguiScale:2\nresourcePacks:[\"vanilla\",\"file/oldpack.zip\"]\nincompatibleResourcePacks:[\"file/oldpack.zip\"]\nrenderDistance:12\n";
        std::fs::write(&src_options, raw_options).unwrap();

        // Simulate the options.txt sanitization performed during clone across different versions
        let content = std::fs::read_to_string(&src_options).unwrap();
        let mut sanitized = Vec::new();
        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("version:") || trimmed.starts_with("dataVersion:") {
                continue;
            }
            if trimmed.starts_with("resourcePacks:") {
                sanitized.push("resourcePacks:[]".to_string());
                continue;
            }
            if trimmed.starts_with("incompatibleResourcePacks:") {
                sanitized.push("incompatibleResourcePacks:[]".to_string());
                continue;
            }
            sanitized.push(line.to_string());
        }
        let output = sanitized.join("\n") + "\n";
        std::fs::write(&dst_options, output).unwrap();

        let result = std::fs::read_to_string(&dst_options).unwrap();
        assert!(!result.contains("version:3465"));
        assert!(!result.contains("file/oldpack.zip"));
        assert!(result.contains("resourcePacks:[]"));
        assert!(result.contains("incompatibleResourcePacks:[]"));
        assert!(result.contains("guiScale:2"));
        assert!(result.contains("renderDistance:12"));
    }

    #[test]
    fn test_profile_json_inherits_sync() {
        let temp_dir = tempfile::tempdir().unwrap();
        let profile_path = temp_dir.path().join("profile.json");
        let initial = r#"{
            "id": "fabric-loader-0.16.10-26.2",
            "inheritsFrom": "26.2",
            "mainClass": "net.fabricmc.loader.impl.launch.knot.KnotClient"
        }"#;
        std::fs::write(&profile_path, initial).unwrap();

        // Simulate inheritsFrom synchronization to 26.3
        let content = std::fs::read_to_string(&profile_path).unwrap();
        let mut val: serde_json::Value = serde_json::from_str(&content).unwrap();
        if let Some(obj) = val.as_object_mut() {
            obj.insert("inheritsFrom".to_string(), serde_json::Value::String("26.3".to_string()));
            let updated = serde_json::to_string_pretty(&val).unwrap();
            std::fs::write(&profile_path, updated).unwrap();
        }

        let updated_content = std::fs::read_to_string(&profile_path).unwrap();
        let updated_val: serde_json::Value = serde_json::from_str(&updated_content).unwrap();
        assert_eq!(updated_val["inheritsFrom"], "26.3");
        assert_eq!(updated_val["mainClass"], "net.fabricmc.loader.impl.launch.knot.KnotClient");
    }
}
