// RixLauncher – Native Wayland Drag & Drop via raw C API
//
// Strategy:
//   - Attach to iced's wl_display (system libwayland, "system" feature)
//   - Create a PRIVATE event queue on that display
//   - Bind seat + data_device_manager + data_device on our queue
//   - Drain our private queue from iced's main thread via poll_dnd()
//
// All proxy operations happen on iced's main thread → no threading races.

#![cfg(target_os = "linux")]
#![allow(non_camel_case_types)]
#![allow(unsafe_op_in_unsafe_fn)]
#![allow(dead_code)]

use std::ffi::{CStr, CString, c_void};
use std::os::raw::c_int;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

#[derive(Copy, Clone)]
struct SendPtr(*mut c_void);
unsafe impl Send for SendPtr {}
unsafe impl Sync for SendPtr {}

// ── Public API ────────────────────────────────────────────────────────

static DROPPED_PATHS: Mutex<Vec<PathBuf>> = Mutex::new(Vec::new());

struct DndState {
    display: *mut c_void,
    queue: *mut c_void,
}
unsafe impl Send for DndState {}

static DND: Mutex<Option<DndState>> = Mutex::new(None);

/// Function-pointer type of libwayland's wl_dispatcher_func_t.
type WlDispatcherFn = unsafe extern "C" fn(
    *const c_void, // implementation
    *mut c_void,   // user_data
    u32,           // opcode
    *const c_void, // wl_message*
    *mut wl_argument,
) -> i32;

/// Saved state of SCTK's original data_device dispatcher so we can forward
/// all events to it (important for clipboard, primary selection, etc.).
struct SctKDdState {
    dispatcher: WlDispatcherFn,
    implementation: *const c_void,
    user_data: *mut c_void,
}
unsafe impl Send for SctKDdState {}

static SCTK_DD_STATE: Mutex<Option<SctKDdState>> = Mutex::new(None);

/// Called from `apply_blur_task` with iced's wl_display pointer.
/// Sets up our private queue on the first call; subsequent calls are no-ops.
pub unsafe fn init_wayland_dnd(display_ptr: *mut c_void) {
    if display_ptr.is_null() {
        return;
    }
    if let Ok(lock) = DND.lock() {
        if lock.is_some() {
            return;
        } // already initialised
    }
    dnd_init(display_ptr);
}

/// Called periodically from iced's update loop (main thread).
/// Flushes outgoing requests, does a non-blocking read of the Wayland socket,
/// then drains our private queue.  All on the main thread → no proxy races.
pub fn poll_dnd() {
    let (display, queue) = {
        let lock = match DND.lock() {
            Ok(g) => g,
            Err(_) => return,
        };
        match lock.as_ref() {
            Some(s) => (s.display, s.queue),
            None => return,
        }
    };

    unsafe {
        let f = fns();

        // 1. Flush any pending outgoing requests
        (f.display_flush)(display);

        // 2. Non-blocking read from the Wayland socket so that events sent
        //    by the compositor while iced was idle land in our queue buffer.
        let fd = (f.display_get_fd)(display);
        if fd >= 0 {
            let mut pfd = libc::pollfd {
                fd,
                events: libc::POLLIN,
                revents: 0,
            };
            let ready = libc::poll(&mut pfd as *mut _, 1, 0);
            if ready > 0 && (pfd.revents & libc::POLLIN) != 0 {
                if (f.display_prepare_read)(display) == 0 {
                    (f.display_read_events)(display);
                } else {
                    eprintln!("[DnD] prepare_read failed (another thread holds lock)");
                }
            }
        }

        // 3. Dispatch any events now in our private queue buffer
        let dispatched = (f.display_dispatch_queue_pending)(display, queue);
        if dispatched > 0 {
            eprintln!("[DnD] dispatched {dispatched} events");
        }

        // 4. Process drop data that reader threads finished
        process_pending_results(display);
    }
}

pub fn take_dropped_paths() -> Vec<PathBuf> {
    if let Ok(mut lock) = DROPPED_PATHS.lock() {
        std::mem::take(&mut *lock)
    } else {
        Vec::new()
    }
}

// ── Types ─────────────────────────────────────────────────────────────

type wl_fixed_t = i32;

#[repr(C)]
#[derive(Copy, Clone)]
pub union wl_argument {
    pub i: i32,
    pub u: u32,
    pub f: i32,
    pub s: *const std::os::raw::c_char,
    pub o: *mut c_void,
    pub n: u32,
    pub a: *const c_void,
    pub h: i32,
}

// ── C function pointers ────────────────────────────────────────────────

struct WlFns {
    display_create_queue: unsafe extern "C" fn(*mut c_void) -> *mut c_void,
    display_roundtrip_queue: unsafe extern "C" fn(*mut c_void, *mut c_void) -> c_int,
    display_dispatch_queue_pending: unsafe extern "C" fn(*mut c_void, *mut c_void) -> c_int,
    display_flush: unsafe extern "C" fn(*mut c_void) -> c_int,
    display_get_fd: unsafe extern "C" fn(*mut c_void) -> c_int,
    display_prepare_read: unsafe extern "C" fn(*mut c_void) -> c_int,
    display_read_events: unsafe extern "C" fn(*mut c_void) -> c_int,
    display_cancel_read: unsafe extern "C" fn(*mut c_void),
    proxy_create_wrapper: unsafe extern "C" fn(*mut c_void) -> *mut c_void,
    proxy_wrapper_destroy: unsafe extern "C" fn(*mut c_void),
    proxy_set_queue: unsafe extern "C" fn(*mut c_void, *mut c_void),
    proxy_marshal_array_constructor:
        unsafe extern "C" fn(*mut c_void, u32, *const wl_argument, *const c_void) -> *mut c_void,
    proxy_marshal_array_constructor_versioned: unsafe extern "C" fn(
        *mut c_void,
        u32,
        *const wl_argument,
        *const c_void,
        u32,
    ) -> *mut c_void,
    proxy_add_listener: unsafe extern "C" fn(*mut c_void, *const c_void, *mut c_void) -> c_int,
    proxy_marshal_array: unsafe extern "C" fn(*mut c_void, u32, *const wl_argument),
    proxy_destroy: unsafe extern "C" fn(*mut c_void),
    proxy_get_id: unsafe extern "C" fn(*mut c_void) -> u32,
    proxy_get_class: unsafe extern "C" fn(*mut c_void) -> *const std::os::raw::c_char,
}

unsafe impl Send for WlFns {}
unsafe impl Sync for WlFns {}

static FNS: OnceLock<WlFns> = OnceLock::new();

fn fns() -> &'static WlFns {
    FNS.get_or_init(|| unsafe {
        let h = lib_handle();
        WlFns {
            display_create_queue: sym(h, "wl_display_create_queue"),
            display_roundtrip_queue: sym(h, "wl_display_roundtrip_queue"),
            display_dispatch_queue_pending: sym(h, "wl_display_dispatch_queue_pending"),
            display_flush: sym(h, "wl_display_flush"),
            display_get_fd: sym(h, "wl_display_get_fd"),
            display_prepare_read: sym(h, "wl_display_prepare_read"),
            display_read_events: sym(h, "wl_display_read_events"),
            display_cancel_read: sym(h, "wl_display_cancel_read"),
            proxy_create_wrapper: sym(h, "wl_proxy_create_wrapper"),
            proxy_wrapper_destroy: sym(h, "wl_proxy_wrapper_destroy"),
            proxy_set_queue: sym(h, "wl_proxy_set_queue"),
            proxy_marshal_array_constructor: sym(h, "wl_proxy_marshal_array_constructor"),
            proxy_marshal_array_constructor_versioned: sym(
                h,
                "wl_proxy_marshal_array_constructor_versioned",
            ),
            proxy_add_listener: sym(h, "wl_proxy_add_listener"),
            proxy_marshal_array: sym(h, "wl_proxy_marshal_array"),
            proxy_destroy: sym(h, "wl_proxy_destroy"),
            proxy_get_id: sym(h, "wl_proxy_get_id"),
            proxy_get_class: sym(h, "wl_proxy_get_class"),
        }
    })
}

static LIB_HANDLE: OnceLock<SendPtr> = OnceLock::new();

fn lib_handle() -> *mut c_void {
    LIB_HANDLE
        .get_or_init(|| unsafe {
            for name in &["libwayland-client.so.0\0", "libwayland-client.so\0"] {
                let p = libc::dlopen(
                    name.as_ptr() as *const _,
                    libc::RTLD_LAZY | libc::RTLD_NOLOAD,
                );
                if !p.is_null() {
                    return SendPtr(p);
                }
            }
            for name in &["libwayland-client.so.0\0", "libwayland-client.so\0"] {
                let p = libc::dlopen(name.as_ptr() as *const _, libc::RTLD_LAZY);
                if !p.is_null() {
                    return SendPtr(p);
                }
            }
            panic!("[DnD] cannot dlopen libwayland-client");
        })
        .0
}

unsafe fn sym<T>(h: *mut c_void, name: &str) -> T {
    let cs = CString::new(name).unwrap();
    let p = libc::dlsym(h, cs.as_ptr());
    assert!(!p.is_null(), "[DnD] dlsym({name}) failed");
    std::mem::transmute_copy(&p)
}

unsafe fn iface(name: &str) -> *const c_void {
    let cs = CString::new(name).unwrap();
    let mut p = libc::dlsym(lib_handle(), cs.as_ptr());
    if p.is_null() {
        p = libc::dlsym(libc::RTLD_DEFAULT, cs.as_ptr());
    }
    assert!(!p.is_null(), "[DnD] interface {name} not found");
    p
}

// ── Initialisation (called once from main thread) ────────────────────

/// Scan libwayland's internal proxy table to find an existing wl_seat proxy.
///
/// Validation: we look for our OWN seat pointer (just created during init) in
/// the scanned table.  Once we find the correct `client_entries.data` offset
/// (i.e. the one that actually contains `our_seat`), we scan the same table
/// for any wl_seat proxy with a LOWER ID – that is iced's seat, the one KWin
/// uses for DnD focus.
///
/// All reads use `ptr::read_unaligned`; no alignment assumptions are made.
unsafe fn find_iced_seat(display_ptr: *mut c_void, our_seat: *mut c_void) -> *mut c_void {
    let f = fns();

    // Use the public API to get our seat's real client-side ID.
    // This is more reliable than reading offset 16 directly.
    let our_id = (f.proxy_get_id)(our_seat);
    eprintln!("[DnD] our_seat id={our_id} ptr={our_seat:?}");

    let bytes = display_ptr as *const u8;

    // Scan every 8-byte-aligned offset in the wl_display struct looking for
    // `client_entries.data` (a void** array of wl_proxy* indexed by object ID).
    //
    // Validation: the canonical check is entries[our_id] == our_seat.
    // This is extremely strong – our_id is a known value, our_seat is a known
    // pointer, and together they uniquely identify the correct offset.
    for data_offset in (120usize..500).step_by(8) {
        let data_ptr: *const u8 =
            std::ptr::read_unaligned(bytes.add(data_offset) as *const *const u8);

        let data_addr = data_ptr as usize;
        if data_addr < 0x1000 || data_addr > 0x7fff_ffff_ffff {
            continue;
        }

        // Validate: entries[our_id] must equal our_seat.
        let entry_at_our_id: usize =
            std::ptr::read_unaligned(data_ptr.add(our_id as usize * 8) as *const usize);
        if entry_at_our_id != our_seat as usize {
            continue;
        }

        // Also sanity-check: entries[0] should be 0 or 1 (null / ID=0 reserved).
        let entry0: usize = std::ptr::read_unaligned(data_ptr as *const usize);
        if entry0 > 4 {
            continue;
        }

        eprintln!("[DnD] proxy-table confirmed at data_offset={data_offset} (our_id={our_id})");

        // Read the total number of entries from client_entries.size
        // (at data_offset - 16, = 8 bytes for size field).
        // This tells us the FULL extent of the proxy table.
        // CRITICAL: seat#73 (SCTK's seat) has ID=73 > our_id=50, so we MUST
        // scan past our_id to find it.
        let entries_size: usize =
            std::ptr::read_unaligned(bytes.add(data_offset - 16) as *const usize);
        let num_entries = ((entries_size / 8) as u32).max(our_id + 1).min(512) as usize;

        // Among all seats that are NOT ours, KWin sends DnD events to the one
        // whose binding received the pointer button press first.
        // Empirically this is always the HIGHEST-ID seat (SCTK's binding,
        // created after winit's initial seat#9 binding but with a higher version).
        // So: find the seat with the HIGHEST ID, excluding our own seat.
        let mut best_ptr: *mut c_void = std::ptr::null_mut();
        let mut best_id: u32 = 0;

        for idx in 2..num_entries {
            let entry_val: usize = std::ptr::read_unaligned(data_ptr.add(idx * 8) as *const usize);
            if entry_val == 0 || entry_val & 1 != 0 {
                continue;
            }

            let proxy = entry_val as *mut c_void;
            if proxy == our_seat {
                continue;
            }

            let class_ptr = (f.proxy_get_class)(proxy);
            if class_ptr.is_null() {
                continue;
            }
            let class = std::ffi::CStr::from_ptr(class_ptr);
            if class.to_bytes() != b"wl_seat" {
                continue;
            }

            let id = (f.proxy_get_id)(proxy);
            eprintln!("[DnD] found wl_seat id={id} ptr={proxy:?}");

            // Pick the seat with the HIGHEST ID (SCTK's seat, not the original seat#9).
            if id > best_id {
                best_id = id;
                best_ptr = proxy;
            }
        }

        if !best_ptr.is_null() {
            eprintln!("[DnD] will use seat id={best_id} (highest-id seat) for data_device");
        } else {
            eprintln!("[DnD] no other wl_seat found in entries 2..{num_entries}");
        }
        return best_ptr;
    }

    eprintln!("[DnD] entries[our_id] never matched our_seat; falling back to our own seat");
    std::ptr::null_mut()
}

unsafe fn dnd_init(display: *mut c_void) {
    let f = fns();

    // 1. Create private event queue on iced's connection
    let queue = (f.display_create_queue)(display);
    if queue.is_null() {
        eprintln!("[DnD] failed to create event queue");
        return;
    }

    // 2. Wrap display so new objects go onto our queue
    let wrapper = (f.proxy_create_wrapper)(display);
    if wrapper.is_null() {
        eprintln!("[DnD] failed to create display wrapper");
        return;
    }
    (f.proxy_set_queue)(wrapper, queue);

    // 3. Get registry
    let registry_iface = iface("wl_registry_interface");
    let args = [wl_argument {
        o: std::ptr::null_mut(),
    }];
    let registry =
        (f.proxy_marshal_array_constructor)(wrapper, 1u32, args.as_ptr(), registry_iface);
    (f.proxy_wrapper_destroy)(wrapper);

    if registry.is_null() {
        eprintln!("[DnD] failed to get registry");
        return;
    }

    // 4. Register global listener
    static REGISTRY_LISTENER: RegistryListener = RegistryListener {
        global: Some(registry_global),
        global_remove: Some(registry_global_remove),
    };
    (f.proxy_add_listener)(
        registry,
        &REGISTRY_LISTENER as *const _ as *const c_void,
        std::ptr::null_mut(),
    );

    // 5. Roundtrip to collect globals (binds our own wl_seat + wl_data_device_manager)
    (f.display_roundtrip_queue)(display, queue);

    let (our_seat, manager) = {
        let g = GLOBALS.lock().unwrap();
        (g.seat, g.manager)
    };
    if our_seat.is_null() || manager.is_null() {
        eprintln!("[DnD] missing globals: seat={our_seat:?} manager={manager:?}");
        return;
    }
    eprintln!("[DnD] bound wl_seat (ours)={our_seat:?} manager={manager:?}");

    // 6. We no longer create our own wl_data_device.
    //    KWin only delivers DnD events to the FIRST data_device registered for
    //    a seat (SCTK's data_device#74).  We hook SCTK's proxy directly via a
    //    dispatcher swap instead of creating a competing data_device that would
    //    generate noisy clipboard-offer objects on our private queue.
    //
    //    our_seat (ID used as proxy-table anchor) is passed as the 'exclude'
    //    sentinel; hook_sctk_data_devices will never see a wl_data_device with
    //    the same id as our_seat (different interface), so it effectively
    //    excludes nothing — all wl_data_device proxies found are SCTK's.
    hook_sctk_data_devices(display, our_seat, our_seat);

    // 7. One more roundtrip so any pending events are flushed.
    (f.display_roundtrip_queue)(display, queue);
    eprintln!("[DnD] initialised – polling via poll_dnd()");

    if let Ok(mut lock) = DND.lock() {
        *lock = Some(DndState { display, queue });
    }
}

/// Dispatcher function we install on SCTK's wl_data_device proxy.
///
/// libwayland calls this for every event on the data_device.  We:
///   1. Forward to SCTK's original dispatcher (preserves clipboard, etc.).
///   2. Handle DnD events ourselves (enter/leave/drop).
///
/// "enter" always tries to accept "text/uri-list" with COPY action.  File
/// managers (Dolphin, Nautilus, Thunar …) always offer this MIME type.
unsafe extern "C" fn our_dd_dispatcher(
    _implementation: *const c_void,
    _data: *mut c_void,
    opcode: u32,
    _message: *const c_void,
    args: *mut wl_argument,
) -> i32 {
    // NOTE: We intentionally do NOT forward to SCTK's original dispatcher.
    // Forwarding causes a crash: SCTK's dispatcher stores tagged (non-8-aligned)
    // pointers in user_data and is not thread-safe — when smithay-clipboard's
    // background thread also dispatches the display it calls our dispatcher and
    // the forwarded call dereferences an unaligned address.
    //
    // Clipboard in iced is handled by smithay-clipboard via its own separate
    // Wayland connection, so SCTK's data_device selection events are not needed
    // for clipboard to work in practice.

    let f = fns();
    match opcode {
        0 => {
            // wl_data_device::data_offer – new offer object announced
            // Reset URI flag; it will be set when we see the offer's MIME types.
            // We can't add our own listener to the offer (SCTK owns it), so
            // we optimistically assume file drags always carry text/uri-list.
            if let Ok(mut h) = OFFER_HAS_URI.lock() {
                *h = true;
            }
        }
        1 => {
            // wl_data_device::enter
            // args layout: serial(u32), surface(obj*), x(fixed), y(fixed), offer(obj*)
            let serial = (*args.add(0)).u;
            let offer = (*args.add(4)).o;
            if offer.is_null() {
                return 0;
            }

            eprintln!("[DnD] ENTER via dispatcher serial={serial} offer={offer:?}");
            if let Ok(mut cur) = CURRENT_OFFER.lock() {
                *cur = Some(SendPtr(offer));
            }

            let mime = CString::new("text/uri-list").unwrap();
            // wl_data_offer.accept (opcode 0): (serial, mime_type)
            (f.proxy_marshal_array)(
                offer,
                0u32,
                [wl_argument { u: serial }, wl_argument { s: mime.as_ptr() }].as_ptr(),
            );
            // wl_data_offer.set_actions (opcode 4): (dnd_actions=copy|move, preferred=copy)
            (f.proxy_marshal_array)(
                offer,
                4u32,
                [
                    wl_argument { u: 3 }, // copy | move
                    wl_argument { u: 1 }, // preferred = copy
                ]
                .as_ptr(),
            );
            eprintln!("[DnD] accepted text/uri-list");
        }
        2 => {
            // wl_data_device::leave
            eprintln!("[DnD] LEAVE via dispatcher");
            if let Ok(mut cur) = CURRENT_OFFER.lock() {
                *cur = None;
            }
        }
        4 => {
            // wl_data_device::drop
            let offer = {
                let mut cur = match CURRENT_OFFER.lock() {
                    Ok(g) => g,
                    Err(_) => return 0,
                };
                cur.take().map(|sp| sp.0)
            };
            if let Some(offer) = offer {
                eprintln!("[DnD] DROP via dispatcher offer={offer:?}");
                // Do NOT call proxy_destroy – SCTK owns the proxy.
                receive_drop_data_no_destroy(offer);
            }
        }
        _ => {}
    }
    0
}

/// Find SCTK's wl_data_device proxy and install our dispatcher on it.
/// We write directly to proxy->dispatcher (offset 56) and save the original.
unsafe fn hook_sctk_data_devices(
    display_ptr: *mut c_void,
    our_seat: *mut c_void,
    our_data_device: *mut c_void,
) {
    let f = fns();
    let bytes = display_ptr as *const u8;
    let our_seat_id = (f.proxy_get_id)(our_seat);
    let our_dd_id = (f.proxy_get_id)(our_data_device);

    for data_offset in (120usize..500).step_by(8) {
        let data_ptr: *const u8 =
            std::ptr::read_unaligned(bytes.add(data_offset) as *const *const u8);
        let data_addr = data_ptr as usize;
        if data_addr < 0x1000 || data_addr > 0x7fff_ffff_ffff {
            continue;
        }

        let entry_at_seat: usize =
            std::ptr::read_unaligned(data_ptr.add(our_seat_id as usize * 8) as *const usize);
        if entry_at_seat != our_seat as usize {
            continue;
        }

        let entries_size: usize =
            std::ptr::read_unaligned(bytes.add(data_offset - 16) as *const usize);
        let num_entries = ((entries_size / 8) as u32).max(our_seat_id + 1).min(512) as usize;

        let mut hooked = 0usize;
        for idx in 2..num_entries {
            let entry_val: usize = std::ptr::read_unaligned(data_ptr.add(idx * 8) as *const usize);
            if entry_val == 0 || entry_val & 1 != 0 {
                continue;
            }

            let proxy = entry_val as *mut c_void;
            let id = (f.proxy_get_id)(proxy);
            if id == our_dd_id {
                continue;
            }

            let class_ptr = (f.proxy_get_class)(proxy);
            if class_ptr.is_null() {
                continue;
            }
            let class = std::ffi::CStr::from_ptr(class_ptr);
            if class.to_bytes() != b"wl_data_device" {
                continue;
            }

            eprintln!("[DnD] hooking SCTK data_device id={id} ptr={proxy:?}");

            // wl_proxy layout (64-bit):
            //   0:  interface*          (wl_object.interface)
            //   8:  implementation*     (wl_object.implementation / C listener)
            //  16:  id u32              (wl_object.id)
            //  24:  display*
            //  32:  queue*
            //  40:  flags u32
            //  44:  refcount i32
            //  48:  user_data*
            //  56:  dispatcher fn*      ← SCTK uses this (Rust dispatch)
            //  64:  version u32
            //  72:  tag**
            let orig_dispatcher: usize = std::ptr::read_unaligned(proxy.add(56) as *const usize);
            let orig_implementation: usize = std::ptr::read_unaligned(proxy.add(8) as *const usize);
            let orig_user_data: *mut c_void =
                std::ptr::read_unaligned(proxy.add(48) as *const *mut c_void);

            eprintln!("[DnD] orig_dispatcher=0x{orig_dispatcher:x} impl=0x{orig_implementation:x}");

            if orig_dispatcher == 0 && orig_implementation == 0 {
                eprintln!("[DnD] no existing dispatcher/implementation – skipping");
                continue;
            }

            // Save SCTK's original dispatcher so we can forward.
            if orig_dispatcher != 0 {
                let disp_fn: WlDispatcherFn = std::mem::transmute(orig_dispatcher);
                if let Ok(mut g) = SCTK_DD_STATE.lock() {
                    *g = Some(SctKDdState {
                        dispatcher: disp_fn,
                        implementation: orig_implementation as *const c_void,
                        user_data: orig_user_data,
                    });
                }
            }

            // Install our dispatcher directly (bypass API guard).
            // Keep implementation and user_data unchanged so SCTK's dispatcher
            // can still use them when forwarded to.
            std::ptr::write_unaligned(
                proxy.add(56) as *mut usize,
                our_dd_dispatcher as *const () as usize,
            );

            hooked += 1;
        }
        eprintln!("[DnD] hooked {hooked} SCTK data_device(s) via dispatcher swap");
        return;
    }
    eprintln!("[DnD] proxy table not found for SCTK hook");
}

// ── Pending drop results (reader threads → main thread) ───────────────

struct PendingDndResult {
    paths: Vec<PathBuf>,
    offer: SendPtr,
}

static PENDING_FINISH: Mutex<Vec<PendingDndResult>> = Mutex::new(Vec::new());
static CURRENT_OFFER: Mutex<Option<SendPtr>> = Mutex::new(None);

fn process_pending_results(display: *mut c_void) {
    let items = {
        let mut pending = match PENDING_FINISH.lock() {
            Ok(g) => g,
            Err(_) => return,
        };
        if pending.is_empty() {
            return;
        }
        std::mem::take(&mut *pending)
    };

    let mut out = Vec::new();
    for item in items {
        unsafe {
            // null sentinel = SCTK-owned offer; SCTK manages its lifecycle.
            // Only call finish/destroy on offers WE created.
            if !item.offer.0.is_null() {
                let f = fns();
                // finish  (opcode 3) – tell compositor DnD was accepted
                (f.proxy_marshal_array)(item.offer.0, 3u32, std::ptr::null());
                // destroy (opcode 2) – Wayland destructor request
                (f.proxy_marshal_array)(item.offer.0, 2u32, std::ptr::null());
                (f.proxy_destroy)(item.offer.0);
                (f.display_flush)(display);
            }
        }
        out.extend(item.paths);
    }

    if !out.is_empty() {
        eprintln!("[DnD] delivering {} paths", out.len());
        if let Ok(mut dropped) = DROPPED_PATHS.lock() {
            dropped.extend(out);
        }
    }
}

// ── Registry listener ─────────────────────────────────────────────────

#[repr(C)]
struct RegistryListener {
    global: Option<
        unsafe extern "C" fn(
            data: *mut c_void,
            registry: *mut c_void,
            name: u32,
            interface: *const std::os::raw::c_char,
            version: u32,
        ),
    >,
    global_remove:
        Option<unsafe extern "C" fn(data: *mut c_void, registry: *mut c_void, name: u32)>,
}

struct DiscoveredGlobals {
    seat: *mut c_void,
    manager: *mut c_void,
}
unsafe impl Send for DiscoveredGlobals {}
static GLOBALS: Mutex<DiscoveredGlobals> = Mutex::new(DiscoveredGlobals {
    seat: std::ptr::null_mut(),
    manager: std::ptr::null_mut(),
});

unsafe extern "C" fn registry_global(
    _data: *mut c_void,
    registry: *mut c_void,
    name: u32,
    interface_name: *const std::os::raw::c_char,
    version: u32,
) {
    let iface_str = CStr::from_ptr(interface_name).to_string_lossy();
    let f = fns();
    let mut g = GLOBALS.lock().unwrap();

    if iface_str == "wl_seat" && g.seat.is_null() {
        let v = version.min(5);
        let args = [
            wl_argument { u: name },
            wl_argument { s: interface_name },
            wl_argument { u: v },
            wl_argument {
                o: std::ptr::null_mut(),
            },
        ];
        let bound = (f.proxy_marshal_array_constructor_versioned)(
            registry,
            0u32,
            args.as_ptr(),
            iface("wl_seat_interface"),
            v,
        );
        if !bound.is_null() {
            g.seat = bound;
            eprintln!("[DnD] bound wl_seat v{v}");
        }
    } else if iface_str == "wl_data_device_manager" && g.manager.is_null() {
        let v = version.min(3);
        let args = [
            wl_argument { u: name },
            wl_argument { s: interface_name },
            wl_argument { u: v },
            wl_argument {
                o: std::ptr::null_mut(),
            },
        ];
        let bound = (f.proxy_marshal_array_constructor_versioned)(
            registry,
            0u32,
            args.as_ptr(),
            iface("wl_data_device_manager_interface"),
            v,
        );
        if !bound.is_null() {
            g.manager = bound;
            eprintln!("[DnD] bound wl_data_device_manager v{v}");
        }
    }
}

unsafe extern "C" fn registry_global_remove(
    _data: *mut c_void,
    _registry: *mut c_void,
    _name: u32,
) {
}

// ── Data offer listener ───────────────────────────────────────────────

#[repr(C)]
struct DataOfferListener {
    offer: Option<unsafe extern "C" fn(*mut c_void, *mut c_void, *const std::os::raw::c_char)>,
    source_actions: Option<unsafe extern "C" fn(*mut c_void, *mut c_void, u32)>,
    action: Option<unsafe extern "C" fn(*mut c_void, *mut c_void, u32)>,
}

static OFFER_LISTENER: DataOfferListener = DataOfferListener {
    offer: Some(offer_mime),
    source_actions: Some(offer_source_actions),
    action: Some(offer_action),
};

static OFFER_HAS_URI: Mutex<bool> = Mutex::new(false);

unsafe extern "C" fn offer_mime(
    _data: *mut c_void,
    _offer: *mut c_void,
    mime: *const std::os::raw::c_char,
) {
    let s = CStr::from_ptr(mime).to_string_lossy();
    if s == "text/uri-list" {
        if let Ok(mut h) = OFFER_HAS_URI.lock() {
            *h = true;
        }
    }
}
unsafe extern "C" fn offer_source_actions(_: *mut c_void, _: *mut c_void, _: u32) {}
unsafe extern "C" fn offer_action(_: *mut c_void, _: *mut c_void, _: u32) {}

// ── Data device listener ──────────────────────────────────────────────

#[repr(C)]
struct DataDeviceListener {
    data_offer: Option<unsafe extern "C" fn(*mut c_void, *mut c_void, *mut c_void)>,
    enter: Option<
        unsafe extern "C" fn(
            *mut c_void,
            *mut c_void,
            u32,
            *mut c_void,
            wl_fixed_t,
            wl_fixed_t,
            *mut c_void,
        ),
    >,
    leave: Option<unsafe extern "C" fn(*mut c_void, *mut c_void)>,
    motion: Option<unsafe extern "C" fn(*mut c_void, *mut c_void, u32, wl_fixed_t, wl_fixed_t)>,
    drop: Option<unsafe extern "C" fn(*mut c_void, *mut c_void)>,
    selection: Option<unsafe extern "C" fn(*mut c_void, *mut c_void, *mut c_void)>,
}

unsafe extern "C" fn dd_data_offer(_data: *mut c_void, _device: *mut c_void, offer: *mut c_void) {
    if offer.is_null() {
        return;
    }
    if let Ok(mut h) = OFFER_HAS_URI.lock() {
        *h = false;
    }
    let f = fns();
    (f.proxy_add_listener)(
        offer,
        &OFFER_LISTENER as *const _ as *const c_void,
        std::ptr::null_mut(),
    );
}

unsafe extern "C" fn dd_enter(
    _data: *mut c_void,
    _device: *mut c_void,
    serial: u32,
    _surface: *mut c_void,
    _x: wl_fixed_t,
    _y: wl_fixed_t,
    offer: *mut c_void,
) {
    eprintln!("[DnD] ENTER serial={serial} offer={offer:?}");
    if offer.is_null() {
        return;
    }

    let has_uri = OFFER_HAS_URI.lock().map(|g| *g).unwrap_or(false);
    if !has_uri {
        eprintln!("[DnD] enter: no text/uri-list, ignoring");
        return;
    }

    if let Ok(mut cur) = CURRENT_OFFER.lock() {
        *cur = Some(SendPtr(offer));
    }

    let f = fns();
    let mime = CString::new("text/uri-list").unwrap();
    // accept(serial, mime) opcode 0
    (f.proxy_marshal_array)(
        offer,
        0u32,
        [wl_argument { u: serial }, wl_argument { s: mime.as_ptr() }].as_ptr(),
    );
    // set_actions(dnd_actions=copy, preferred=copy) opcode 4
    (f.proxy_marshal_array)(
        offer,
        4u32,
        [wl_argument { u: 1 }, wl_argument { u: 1 }].as_ptr(),
    );
    eprintln!("[DnD] accepted text/uri-list");
}

unsafe extern "C" fn dd_leave(_data: *mut c_void, _device: *mut c_void) {
    eprintln!("[DnD] LEAVE");
    if let Ok(mut cur) = CURRENT_OFFER.lock() {
        *cur = None;
    }
}

unsafe extern "C" fn dd_motion(
    _data: *mut c_void,
    _device: *mut c_void,
    _time: u32,
    _x: wl_fixed_t,
    _y: wl_fixed_t,
) {
}

unsafe extern "C" fn dd_drop(_data: *mut c_void, _device: *mut c_void) {
    let offer = {
        let mut cur = match CURRENT_OFFER.lock() {
            Ok(g) => g,
            Err(_) => return,
        };
        cur.take().map(|sp| sp.0)
    };
    let Some(offer) = offer else {
        eprintln!("[DnD] drop: no current offer");
        return;
    };
    eprintln!("[DnD] DROP! offer={offer:?}");
    receive_drop_data(offer);
}

unsafe extern "C" fn dd_selection(_data: *mut c_void, _device: *mut c_void, offer: *mut c_void) {
    // We only care about DnD, not clipboard selection.
    // Properly destroy the offer to avoid leaking proxy objects.
    if !offer.is_null() {
        let f = fns();
        // destroy opcode 2 (Wayland protocol destructor)
        (f.proxy_marshal_array)(offer, 2u32, std::ptr::null());
        (f.proxy_destroy)(offer);
    }
}

// ── Data reception (spawns reader thread, result queued for main thread) ─

fn receive_drop_data(offer: *mut c_void) {
    use std::os::unix::net::UnixStream;
    let (reader, writer) = match UnixStream::pair() {
        Ok(p) => p,
        Err(e) => {
            eprintln!("[DnD] socketpair failed: {e}");
            return;
        }
    };
    let write_fd = {
        use std::os::fd::IntoRawFd;
        writer.into_raw_fd()
    };

    unsafe {
        let f = fns();
        let mime = CString::new("text/uri-list").unwrap();
        // receive(mime, fd) opcode 1
        (f.proxy_marshal_array)(
            offer,
            1u32,
            [
                wl_argument { s: mime.as_ptr() },
                wl_argument { h: write_fd },
            ]
            .as_ptr(),
        );
        libc::close(write_fd);
    }

    let offer_sp = SendPtr(offer);
    std::thread::spawn(move || {
        use std::io::Read;
        let mut reader = reader;
        let _ = reader.set_read_timeout(Some(std::time::Duration::from_secs(5)));

        let mut buf = Vec::new();
        let mut tmp = [0u8; 4096];
        loop {
            match reader.read(&mut tmp) {
                Ok(0) => break,
                Ok(n) => buf.extend_from_slice(&tmp[..n]),
                Err(ref e)
                    if matches!(
                        e.kind(),
                        std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
                    ) =>
                {
                    break;
                }
                Err(_) => break,
            }
        }

        let content = String::from_utf8_lossy(&buf);
        eprintln!("[DnD] received {} bytes: {:?}", buf.len(), content);

        let paths: Vec<PathBuf> = content
            .lines()
            .map(str::trim)
            .filter(|l| !l.is_empty() && !l.starts_with('#'))
            .filter_map(|l| l.strip_prefix("file://"))
            .map(|uri| PathBuf::from(url_decode(uri)))
            .collect();

        eprintln!("[DnD] decoded {} paths: {:?}", paths.len(), paths);

        if let Ok(mut pending) = PENDING_FINISH.lock() {
            pending.push(PendingDndResult {
                paths,
                offer: offer_sp,
            });
        }
    });
}

/// Variant for SCTK-owned offers: sends finish but does NOT call proxy_destroy.
/// SCTK is responsible for destroying its own wl_data_offer proxy.
fn receive_drop_data_no_destroy(offer: *mut c_void) {
    use std::os::unix::net::UnixStream;
    let (reader, writer) = match UnixStream::pair() {
        Ok(p) => p,
        Err(e) => {
            eprintln!("[DnD] socketpair failed: {e}");
            return;
        }
    };
    let write_fd = {
        use std::os::fd::IntoRawFd;
        writer.into_raw_fd()
    };

    // Flush so compositor sees our receive request and starts writing.
    let display = DND
        .lock()
        .ok()
        .and_then(|g| g.as_ref().map(|s| s.display))
        .unwrap_or(std::ptr::null_mut());

    unsafe {
        let f = fns();
        let mime = CString::new("text/uri-list").unwrap();
        (f.proxy_marshal_array)(
            offer,
            1u32,
            [
                wl_argument { s: mime.as_ptr() },
                wl_argument { h: write_fd },
            ]
            .as_ptr(),
        );
        libc::close(write_fd);
        if !display.is_null() {
            (f.display_flush)(display);
        }
    }

    std::thread::spawn(move || {
        use std::io::Read;
        let mut reader = reader;
        let _ = reader.set_read_timeout(Some(std::time::Duration::from_secs(5)));

        let mut buf = Vec::new();
        let mut tmp = [0u8; 4096];
        loop {
            match reader.read(&mut tmp) {
                Ok(0) => break,
                Ok(n) => buf.extend_from_slice(&tmp[..n]),
                Err(ref e)
                    if matches!(
                        e.kind(),
                        std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
                    ) =>
                {
                    break;
                }
                Err(_) => break,
            }
        }

        let content = String::from_utf8_lossy(&buf);
        eprintln!("[DnD] received {} bytes: {:?}", buf.len(), content);

        let paths: Vec<PathBuf> = content
            .lines()
            .map(str::trim)
            .filter(|l| !l.is_empty() && !l.starts_with('#'))
            .filter_map(|l| l.strip_prefix("file://"))
            .map(|uri| PathBuf::from(url_decode(uri)))
            .collect();

        eprintln!("[DnD] decoded {} paths: {:?}", paths.len(), paths);

        // Use null sentinel to skip finish/destroy in process_pending_results
        // (SCTK manages the offer proxy's lifetime).
        if let Ok(mut pending) = PENDING_FINISH.lock() {
            pending.push(PendingDndResult {
                paths,
                offer: SendPtr(std::ptr::null_mut()),
            });
        }
    });
}

// ── URL decoding ──────────────────────────────────────────────────────

fn url_decode(s: &str) -> String {
    let mut out = Vec::with_capacity(s.len());
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let Ok(h) =
                u8::from_str_radix(std::str::from_utf8(&bytes[i + 1..=i + 2]).unwrap_or(""), 16)
            {
                out.push(h);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).to_string()
}
