#![windows_subsystem = "windows"]
// RixLauncher Main Entrypoint
// Configures and starts the glassmorphic Rust + Iced application.

mod blur;
mod icons;
mod launcher;
mod style;
#[cfg(target_os = "linux")]
pub mod wayland_dnd;

use launcher::{RixLauncher, constants};

fn theme(_state: &RixLauncher) -> iced::Theme {
    iced::Theme::custom(
        "RixTheme".to_string(),
        iced::theme::Palette {
            background: iced::Color::TRANSPARENT,
            ..iced::Theme::Dark.palette()
        },
    )
}

fn title(_state: &RixLauncher) -> String {
    constants::APP_NAME.to_string()
}

fn window_settings(transparent: bool, blur: bool) -> iced::window::Settings {
    let icon = {
        const ICON_RGBA: &[u8] = include_bytes!("icon_rgba.bin");
        iced::window::icon::from_rgba(ICON_RGBA.to_vec(), 256, 256).ok()
    };

    #[allow(unused_mut)]
    let mut settings = iced::window::Settings {
        size: (
            constants::DEFAULT_WINDOW_WIDTH,
            constants::DEFAULT_WINDOW_HEIGHT,
        )
            .into(),
        min_size: Some((constants::MIN_WINDOW_WIDTH, constants::MIN_WINDOW_HEIGHT).into()),
        position: iced::window::Position::Centered,
        resizable: true,
        decorations: false,
        transparent,
        blur,
        icon,
        ..Default::default()
    };

    #[cfg(target_os = "linux")]
    {
        settings.platform_specific.application_id = constants::APPLICATION_ID.to_string();
    }

    settings
}

fn write_diagnostic(contents: impl AsRef<[u8]>) {
    let path = launcher::get_rixlauncher_root()
        .join("logs")
        .join("launcher-crash.log");
    let _ = launcher::utils::append_rotating_log(
        &path,
        contents.as_ref(),
        constants::MAX_LAUNCHER_LOG_BYTES,
    );
}

fn main() {
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        // Print the panic to stderr using the default hook
        default_hook(info);

        let msg = match info.payload().downcast_ref::<&str>() {
            Some(s) => *s,
            None => match info.payload().downcast_ref::<String>() {
                Some(s) => &**s,
                None => "Box<Any>",
            },
        };
        let location = info
            .location()
            .map(|loc| format!("{}:{}:{}", loc.file(), loc.line(), loc.column()))
            .unwrap_or_else(|| "unknown location".to_string());
        let crash_content = format!(
            "{} Panicked!\nMessage: {}\nLocation: {}\n",
            constants::APP_NAME,
            msg,
            location
        );
        write_diagnostic(crash_content.as_bytes());
    }));

    #[cfg(target_os = "windows")]
    {
        // DX12's HWND swapchain is opaque, and tiny-skia/softbuffer does not
        // guarantee window alpha. Prefer Vulkan so the translucent UI can
        // expose DWM blur, retaining iced's software fallback if unavailable.
        // Respect explicit renderer/backend overrides.
        if std::env::var_os("ICED_BACKEND").is_none() && std::env::var_os("WGPU_BACKEND").is_none()
        {
            // SAFETY: startup is still single-threaded, before iced/Tokio.
            unsafe {
                std::env::set_var("WGPU_BACKEND", "vulkan");
            }
        }
    }

    #[cfg(target_os = "linux")]
    blur::clear_legacy_kwin_rules_blur();

    // Keep the glass window as the primary presentation on platforms that
    // support it. The platform task applies KDE/Windows native blur once the
    // native surface exists; the two fallbacks keep startup reliable.
    let run_res = iced::application(RixLauncher::new, RixLauncher::update, RixLauncher::view)
        .title(title)
        .window(window_settings(true, true))
        .subscription(RixLauncher::subscription)
        .theme(theme)
        .run();

    if let Err(e) = run_res {
        write_diagnostic(format!("Transparent window failed: {:?}\n", e).as_bytes());

        // Retry without the compositor's built-in blur request.
        let run_res2 = iced::application(RixLauncher::new, RixLauncher::update, RixLauncher::view)
            .title(title)
            .window(window_settings(true, false))
            .subscription(RixLauncher::subscription)
            .theme(theme)
            .run();

        if let Err(e2) = run_res2 {
            write_diagnostic(
                format!("Transparent window without blur failed: {:?}\n", e2).as_bytes(),
            );

            let run_res3 =
                iced::application(RixLauncher::new, RixLauncher::update, RixLauncher::view)
                    .title(title)
                    .window(window_settings(false, false))
                    .subscription(RixLauncher::subscription)
                    .theme(theme)
                    .run();

            if let Err(e3) = run_res3 {
                write_diagnostic(format!("All window modes failed: {:?}\n", e3).as_bytes());
            }
        }
    }
}
