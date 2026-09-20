// RixLauncher – KDE Wayland blur-behind support
// Uses the org_kde_kwin_blur Wayland protocol to request compositor blur.
// Works on KDE Plasma/KWin with the "Blur" effect enabled.

#![cfg(target_os = "linux")]
#![allow(dead_code)]

use std::cell::RefCell;
use std::ffi::c_void;
use std::mem::ManuallyDrop;

use wayland_client::{
    Connection, Dispatch, EventQueue, Proxy, QueueHandle,
    backend::Backend,
    globals::{GlobalListContents, registry_queue_init},
    protocol::{wl_compositor, wl_region, wl_registry, wl_surface},
};
use wayland_protocols::ext::background_effect::v1::client::{
    ext_background_effect_manager_v1::ExtBackgroundEffectManagerV1,
    ext_background_effect_surface_v1::ExtBackgroundEffectSurfaceV1,
};
use wayland_protocols_plasma::blur::client::{
    org_kde_kwin_blur::OrgKdeKwinBlur, org_kde_kwin_blur_manager::OrgKdeKwinBlurManager,
};

struct BlurState;

thread_local! {
    static BLUR_HANDLE: RefCell<Option<BlurHandle>> = const { RefCell::new(None) };
}

enum BlurEffect {
    Ext(ExtBackgroundEffectSurfaceV1),
    Legacy(OrgKdeKwinBlur),
}

struct BlurHandle {
    // The display belongs to iced/winit. Do not let the secondary wayland
    // connection try to destroy that foreign display during application exit.
    conn: ManuallyDrop<Connection>,
    queue: ManuallyDrop<EventQueue<BlurState>>,
    compositor: wl_compositor::WlCompositor,
    surface: wl_surface::WlSurface,
    effect: BlurEffect,
    region: Option<wl_region::WlRegion>,
    surface_ptr: *mut c_void,
}

impl Drop for BlurHandle {
    fn drop(&mut self) {
        // SAFETY: both objects wrap iced's foreign wl_display. Forgetting the
        // destructors avoids double-freeing the display during TLS teardown.
        unsafe {
            let conn = ManuallyDrop::take(&mut self.conn);
            let queue = ManuallyDrop::take(&mut self.queue);
            std::mem::forget(conn);
            std::mem::forget(queue);
        }
    }
}

impl Dispatch<wl_registry::WlRegistry, GlobalListContents> for BlurState {
    fn event(
        _: &mut Self,
        _: &wl_registry::WlRegistry,
        _: wl_registry::Event,
        _: &GlobalListContents,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<wl_compositor::WlCompositor, ()> for BlurState {
    fn event(
        _: &mut Self,
        _: &wl_compositor::WlCompositor,
        _: wl_compositor::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<wl_region::WlRegion, ()> for BlurState {
    fn event(
        _: &mut Self,
        _: &wl_region::WlRegion,
        _: wl_region::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<OrgKdeKwinBlurManager, ()> for BlurState {
    fn event(
        _: &mut Self,
        _: &OrgKdeKwinBlurManager,
        _: <OrgKdeKwinBlurManager as Proxy>::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<OrgKdeKwinBlur, ()> for BlurState {
    fn event(
        _: &mut Self,
        _: &OrgKdeKwinBlur,
        _: <OrgKdeKwinBlur as Proxy>::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<ExtBackgroundEffectManagerV1, ()> for BlurState {
    fn event(
        _: &mut Self,
        _: &ExtBackgroundEffectManagerV1,
        _: <ExtBackgroundEffectManagerV1 as Proxy>::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<ExtBackgroundEffectSurfaceV1, ()> for BlurState {
    fn event(
        _: &mut Self,
        _: &ExtBackgroundEffectSurfaceV1,
        _: <ExtBackgroundEffectSurfaceV1 as Proxy>::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

/// Apply KDE Wayland blur-behind to the window surface.
pub unsafe fn apply_kde_wayland_blur(
    display_ptr: *mut c_void,
    surface_ptr: *mut c_void,
    width: i32,
    height: i32,
    corner_radius: i32,
) {
    BLUR_HANDLE.with(|cell| {
        let mut handle = cell.borrow_mut();
        if let Some(existing) = handle.as_mut() {
            if existing.surface_ptr == surface_ptr {
                existing.set_region(width, height, corner_radius);
                return;
            }
            if let Some(old_region) = existing.region.take() {
                old_region.destroy();
            }
            *handle = None;
        }

        match unsafe { BlurHandle::new(display_ptr, surface_ptr) } {
            Ok(mut new_handle) => {
                new_handle.set_region(width, height, corner_radius);
                *handle = Some(new_handle);
            }
            Err(error) => eprintln!("{error}"),
        }
    });
}

impl BlurHandle {
    unsafe fn new(display_ptr: *mut c_void, surface_ptr: *mut c_void) -> Result<Self, String> {
        let backend = unsafe {
            Backend::from_foreign_display(display_ptr as *mut wayland_sys::client::wl_display)
        };
        let conn = Connection::from_backend(backend);
        let (globals, mut queue) = registry_queue_init::<BlurState>(&conn)
            .map_err(|error| format!("[RixLauncher] wayland registry init failed: {error}"))?;
        let qh = queue.handle();
        let mut state = BlurState;
        let compositor: wl_compositor::WlCompositor = globals
            .bind(&qh, 1..=6, ())
            .map_err(|error| format!("[RixLauncher] wl_compositor bind failed: {error}"))?;
        queue
            .roundtrip(&mut state)
            .map_err(|error| format!("[RixLauncher] wayland roundtrip failed: {error}"))?;

        use wayland_client::backend::ObjectId;
        let surface_id = unsafe {
            ObjectId::from_ptr(
                wl_surface::WlSurface::interface(),
                surface_ptr as *mut wayland_sys::client::wl_proxy,
            )
        }
        .map_err(|error| format!("[RixLauncher] could not wrap surface pointer: {error}"))?;
        let surface = wl_surface::WlSurface::from_id(&conn, surface_id)
            .map_err(|error| format!("[RixLauncher] WlSurface::from_id failed: {error}"))?;

        if let Ok(effect_manager) =
            globals.bind::<ExtBackgroundEffectManagerV1, _, _>(&qh, 1..=1, ())
        {
            let effect = effect_manager.get_background_effect(&surface, &qh, ());
            std::mem::forget(effect_manager);
            eprintln!("[RixLauncher] KDE ext_background_effect blur attached.");
            return Ok(Self {
                conn: ManuallyDrop::new(conn),
                queue: ManuallyDrop::new(queue),
                compositor,
                surface,
                effect: BlurEffect::Ext(effect),
                region: None,
                surface_ptr,
            });
        }

        let blur_manager: OrgKdeKwinBlurManager = globals
            .bind(&qh, 1..=1, ())
            .map_err(|_| {
                "[RixLauncher] no KDE blur protocol found. Tried ext_background_effect_manager_v1 and org_kde_kwin_blur_manager.".to_string()
            })?;
        let blur = blur_manager.create(&surface, &qh, ());
        std::mem::forget(blur_manager);
        eprintln!("[RixLauncher] Legacy KDE Wayland blur attached.");
        Ok(Self {
            conn: ManuallyDrop::new(conn),
            queue: ManuallyDrop::new(queue),
            compositor,
            surface,
            effect: BlurEffect::Legacy(blur),
            region: None,
            surface_ptr,
        })
    }

    fn set_region(&mut self, width: i32, height: i32, corner_radius: i32) {
        let qh = self.queue.handle();
        let region = self.compositor.create_region(&qh, ());
        add_rounded_rect_region(&region, width, height, corner_radius);
        match &self.effect {
            BlurEffect::Ext(effect) => {
                effect.set_blur_region(Some(&region));
                self.surface.commit();
            }
            BlurEffect::Legacy(blur) => {
                blur.set_region(Some(&region));
                blur.commit();
                self.surface.commit();
            }
        }
        if let Err(error) = self.conn.flush() {
            eprintln!("[RixLauncher] wayland flush failed: {error}");
        }
        if let Some(old_region) = self.region.take() {
            old_region.destroy();
        }
        self.region = Some(region);
    }
}

fn add_rounded_rect_region(region: &wl_region::WlRegion, width: i32, height: i32, radius: i32) {
    let width = width.max(1);
    let height = height.max(1);
    let radius = radius.max(0).min(width / 2).min(height / 2);
    if radius == 0 {
        region.add(0, 0, width, height);
        return;
    }
    let middle_height = height - radius * 2;
    if middle_height > 0 {
        region.add(0, radius, width, middle_height);
    }
    let r = f64::from(radius);
    let samples = 4usize;
    for y in 0..radius {
        let mut inset_sum = 0.0f64;
        for sample in 0..samples {
            let fy = f64::from(y) + (sample as f64 + 0.5) / samples as f64;
            let dy = r - fy;
            let inset = if dy >= 0.0 {
                (r - (r * r - dy * dy).sqrt()).max(0.0)
            } else {
                0.0
            };
            inset_sum += inset;
        }
        let inset = (inset_sum / samples as f64).floor() as i32;
        let row_width = width - inset * 2;
        if row_width > 0 {
            region.add(inset, y, row_width, 1);
            region.add(inset, height - y - 1, row_width, 1);
        }
    }
}

pub fn clear_legacy_kwin_rules_blur() {
    let description = std::process::Command::new("kreadconfig6")
        .args([
            "--file",
            "kwinrulesrc",
            "--group",
            "1",
            "--key",
            "Description",
        ])
        .output()
        .ok()
        .and_then(|output| String::from_utf8(output.stdout).ok())
        .unwrap_or_default();
    if description.trim() != "RixLauncher Blur" {
        return;
    }
    let kwrite = |key: &str, value: &str| {
        std::process::Command::new("kwriteconfig6")
            .args(["--file", "kwinrulesrc", "--group", "1", "--key", key, value])
            .status()
            .ok()
    };
    kwrite("Description", "RixLauncher Blur Disabled");
    kwrite("blurbehind", "false");
    kwrite("blurbehindmatch", "0");
    kwrite("wmclass", "__rix_launcher_legacy_disabled__");
    let _ = std::process::Command::new("dbus-send")
        .args([
            "--session",
            "--dest=org.kde.KWin",
            "--type=method_call",
            "/KWin",
            "org.kde.KWin.reconfigure",
        ])
        .status();
}
