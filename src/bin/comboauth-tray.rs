//! comboauth-tray — menu bar / system tray launcher/controller for comboauth-daemon.
//!
//! macOS: native NSStatusItem menu bar via tray-icon + objc2.
//! Linux: AppIndicator/GTK system tray via tray-icon + libappindicator3 (runtime dep).

use comboauth::ipc::{DaemonRequest, DaemonResponse, send_request};

/// Tray icon decoded from assets/tray-icon.png (22×22 RGBA).
/// On macOS the icon is loaded as a template image (alpha-only matters).
/// On Linux it appears as-is via AppIndicator.
fn build_icon() -> Result<tray_icon::Icon, Box<dyn std::error::Error>> {
    const ICON_PNG: &[u8] = include_bytes!("../../assets/tray-icon.png");

    let decoder = png::Decoder::new(ICON_PNG);
    let mut reader = decoder.read_info()?;
    let mut buf = vec![0; reader.output_buffer_size()];
    let info = reader.next_frame(&mut buf)?;
    let rgba = match info.color_type {
        png::ColorType::Rgba => buf[..info.buffer_size()].to_vec(),
        other => return Err(format!("tray-icon.png: expected RGBA, got {other:?}").into()),
    };

    Ok(tray_icon::Icon::from_rgba(rgba, info.width, info.height)?)
}

fn handle_menu_event(
    id: &tray_icon::menu::MenuId,
    open_id: &tray_icon::menu::MenuId,
    status_id: &tray_icon::menu::MenuId,
    stop_id: &tray_icon::menu::MenuId,
    quit_id: &tray_icon::menu::MenuId,
) {
    if id == open_id {
        match send_request(&DaemonRequest::ShowTui) {
            Ok(DaemonResponse::Ok) => eprintln!("comboauth-tray: launched TUI"),
            Ok(DaemonResponse::Error { message }) => eprintln!("comboauth-tray: ShowTui failed: {message}"),
            Ok(_) => {}
            Err(e) => eprintln!("comboauth-tray: daemon unreachable: {e}"),
        }
    } else if id == status_id {
        match send_request(&DaemonRequest::Status) {
            Ok(DaemonResponse::Status { running, version }) => {
                eprintln!("comboauth-tray: daemon status — running={running}, version={version}");
            }
            Ok(DaemonResponse::Error { message }) => eprintln!("comboauth-tray: status error: {message}"),
            Ok(_) => {}
            Err(e) => eprintln!("comboauth-tray: daemon unreachable: {e}"),
        }
    } else if id == stop_id {
        match send_request(&DaemonRequest::Stop) {
            Ok(_) => eprintln!("comboauth-tray: daemon stopped"),
            Err(e) => eprintln!("comboauth-tray: failed to stop daemon: {e}"),
        }
    } else if id == quit_id {
        eprintln!("comboauth-tray: quitting");
        std::process::exit(0);
    }
}

#[cfg(target_os = "macos")]
fn run_macos_tray() -> Result<(), Box<dyn std::error::Error>> {
    use tray_icon::menu::{Menu, MenuEvent, MenuItem};
    use objc2::MainThreadMarker;
    use objc2_app_kit::{NSApplication, NSApplicationActivationPolicy};
    use tray_icon::TrayIconBuilder;

    let mtm = MainThreadMarker::new().expect("comboauth-tray must run on the main thread");
    let app = NSApplication::sharedApplication(mtm);
    app.setActivationPolicy(NSApplicationActivationPolicy::Accessory);

    let menu = Menu::new();
    let open_item = MenuItem::new("Open ComboAuth", true, None);
    let status_item = MenuItem::new("Status", true, None);
    let stop_item = MenuItem::new("Stop Daemon", true, None);
    let quit_item = MenuItem::new("Quit", true, None);
    menu.append_items(&[&open_item, &status_item, &stop_item, &quit_item])?;

    let open_id = open_item.id().clone();
    let status_id = status_item.id().clone();
    let stop_id = stop_item.id().clone();
    let quit_id = quit_item.id().clone();

    let _tray = TrayIconBuilder::new()
        .with_menu(Box::new(menu))
        .with_icon(build_icon()?)
        .with_icon_as_template(true)
        .with_tooltip("ComboAuth")
        .build()?;

    std::thread::spawn(move || {
        let receiver = MenuEvent::receiver();
        loop {
            if let Ok(event) = receiver.recv() {
                handle_menu_event(event.id(), &open_id, &status_id, &stop_id, &quit_id);
            }
        }
    });

    app.run();
    Ok(())
}

/// Linux AppIndicator tray via tray-icon + libappindicator3.
/// Requires libayatana-appindicator3-1 or libappindicator3-1 at runtime
/// and a running GTK-compatible desktop environment (X11 or XWayland).
#[cfg(target_os = "linux")]
fn run_linux_tray() -> Result<(), Box<dyn std::error::Error>> {
    use tray_icon::menu::{Menu, MenuEvent, MenuItem};
    use tray_icon::TrayIconBuilder;

    gtk::init().map_err(|e| format!("GTK init failed: {e}"))?;

    let menu = Menu::new();
    let open_item = MenuItem::new("Open ComboAuth", true, None);
    let status_item = MenuItem::new("Status", true, None);
    let stop_item = MenuItem::new("Stop Daemon", true, None);
    let quit_item = MenuItem::new("Quit", true, None);
    menu.append_items(&[&open_item, &status_item, &stop_item, &quit_item])?;

    let open_id = open_item.id().clone();
    let status_id = status_item.id().clone();
    let stop_id = stop_item.id().clone();
    let quit_id = quit_item.id().clone();

    let _tray = TrayIconBuilder::new()
        .with_menu(Box::new(menu))
        .with_icon(build_icon()?)
        .with_tooltip("ComboAuth")
        .build()?;

    // Handle menu events on a background thread so blocking IPC doesn't stall GTK.
    // Quit is special: gtk::main_quit() must run on the GTK-initialized main thread,
    // so we marshal it via glib::MainContext::invoke() and break the receiver loop.
    let quit_id_clone = quit_id.clone();
    std::thread::spawn(move || {
        let receiver = MenuEvent::receiver();
        loop {
            if let Ok(event) = receiver.recv() {
                if event.id() == &quit_id_clone {
                    eprintln!("comboauth-tray: quitting");
                    gtk::glib::MainContext::default().invoke(|| gtk::main_quit());
                    break;
                }
                handle_menu_event(event.id(), &open_id, &status_id, &stop_id, &quit_id);
            }
        }
    });

    gtk::main();
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(target_os = "macos")]
    {
        run_macos_tray()
    }
    #[cfg(target_os = "linux")]
    {
        run_linux_tray()
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    {
        eprintln!("comboauth-tray: unsupported platform");
        Ok(())
    }
}
