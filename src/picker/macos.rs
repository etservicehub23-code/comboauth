//! macOS floating combo picker.
//!
//! Opens a small borderless panel and briefly takes keyboard focus so it can
//! capture a combo sequence without leaking keystrokes into whatever app was
//! previously focused — the same pattern Spotlight/Alfred/Raycast use. No
//! CGEventTap / Input Monitoring permission is needed: while our panel is
//! the key window, the OS routes keyboard input to it via the normal
//! responder chain, and `NSEvent` local monitors observe (and can swallow)
//! those events.
//!
//! Flow: Ctrl+K (handled by the daemon's global hotkey) calls
//! `show_picker_and_capture`. It runs on a background thread, but all
//! AppKit work is marshalled onto the main thread via `dispatch2::run_on_main`
//! since NSWindow/NSEvent monitors must be created on the main thread.

use std::cell::RefCell;
use std::rc::Rc;
use std::time::Instant;

use block2::RcBlock;
use objc2::rc::Retained;
use objc2::MainThreadMarker;
use objc2_app_kit::{
    NSApplication, NSApplicationActivationOptions, NSBackingStoreType, NSColor, NSEvent,
    NSEventMask, NSFloatingWindowLevel, NSPanel, NSWindowStyleMask, NSWorkspace,
};
use objc2_foundation::{NSPoint, NSRect, NSSize};

use crate::combo::Combo;
use crate::focus::{paste_decision, FieldKind, PasteDecision};
use crate::profile::{ComboProfile, ComboProfileId};
use crate::service::{ServiceId, ServiceRegistry};
use crate::vault::SecretStore;

const TIMING_TOLERANCE_PCT: u32 = 40;
/// Clipboard clear delay for `PasteDecision::Refuse`: long enough for the
/// user to manually paste (unlike the near-instant clear after an
/// auto-paste, where the keystroke already happened).
const REFUSE_COPY_CLEAR_MS: u64 = 8_000;
const ARROW_UP: u16 = 126;
const ARROW_DOWN: u16 = 125;
const ARROW_LEFT: u16 = 123;
const ARROW_RIGHT: u16 = 124;
const KEYCODE_RETURN: u16 = 36;
const KEYCODE_ESCAPE: u16 = 53;

enum Outcome {
    Matched { service_id: ServiceId, service_name: String },
    NoServiceForCombo,
    NoMatch,
    Cancelled,
}

/// Captures a combo via a floating panel, matches it, and pastes the
/// resulting secret. Safe to call from any thread — internally dispatches
/// to the main thread.
pub fn show_picker_and_capture(
    profiles: Vec<ComboProfile>,
    registry: ServiceRegistry,
    secret_store: &(dyn SecretStore + Send + Sync),
    field_kind: FieldKind,
) {
    let decision = paste_decision(field_kind);
    let outcome = dispatch2::run_on_main(|mtm| run_picker(mtm, &profiles, &registry, decision, field_kind));

    match outcome {
        Outcome::Matched { service_id, service_name } => match secret_store.get_secret(&service_id) {
            Ok(secret) => {
                let secret_str = String::from_utf8_lossy(secret.expose_bytes()).into_owned();
                if decision == PasteDecision::Refuse {
                    match crate::paste::copy_and_clear(&secret_str, REFUSE_COPY_CLEAR_MS) {
                        Ok(()) => eprintln!(
                            "comboauth-daemon: picker matched '{service_name}', focused field is {field_kind:?} (not editable) -- copied to clipboard instead of auto-pasting"
                        ),
                        Err(e) => eprintln!("comboauth-daemon: picker matched '{service_name}' but clipboard copy failed: {e}"),
                    }
                } else {
                    match crate::paste::paste_and_clear(&secret_str, 200) {
                        Ok(()) => eprintln!("comboauth-daemon: picker matched '{service_name}', pasted"),
                        Err(e) => eprintln!("comboauth-daemon: picker matched '{service_name}' but paste failed: {e}"),
                    }
                }
            }
            Err(e) => eprintln!("comboauth-daemon: picker matched '{service_name}' but secret lookup failed: {e}"),
        },
        Outcome::NoServiceForCombo => eprintln!("comboauth-daemon: picker combo matched, but no service is assigned to it"),
        Outcome::NoMatch => eprintln!("comboauth-daemon: picker combo did not match any profile"),
        Outcome::Cancelled => eprintln!("comboauth-daemon: picker cancelled"),
    }
}

fn run_picker(
    mtm: MainThreadMarker,
    profiles: &[ComboProfile],
    registry: &ServiceRegistry,
    decision: PasteDecision,
    field_kind: FieldKind,
) -> Outcome {
    let workspace = NSWorkspace::sharedWorkspace();
    let previous_app = workspace.frontmostApplication();

    let panel = build_panel(mtm);
    panel.makeKeyAndOrderFront(None);
    NSApplication::sharedApplication(mtm).activate();

    let tokens: Rc<RefCell<Vec<String>>> = Rc::new(RefCell::new(Vec::new()));
    let timestamps: Rc<RefCell<Vec<Instant>>> = Rc::new(RefCell::new(Vec::new()));
    let outcome: Rc<RefCell<Option<Outcome>>> = Rc::new(RefCell::new(None));
    // Set once a combo matches under `PasteDecision::ConfirmFirst`, while we
    // wait for the one extra keypress the policy requires before pasting.
    let awaiting_confirm: Rc<RefCell<bool>> = Rc::new(RefCell::new(false));
    let pending_match: Rc<RefCell<Option<(ServiceId, String)>>> = Rc::new(RefCell::new(None));

    let app = NSApplication::sharedApplication(mtm);

    let monitor = {
        let tokens = tokens.clone();
        let timestamps = timestamps.clone();
        let outcome = outcome.clone();
        let awaiting_confirm = awaiting_confirm.clone();
        let pending_match = pending_match.clone();
        let profiles = profiles.to_vec();
        let registry = registry.clone();
        let app = app.clone();

        let block = RcBlock::new(move |event: std::ptr::NonNull<NSEvent>| -> *mut NSEvent {
            let event_ref = unsafe { event.as_ref() };
            let key_code = event_ref.keyCode();

            if *awaiting_confirm.borrow() {
                match key_code {
                    KEYCODE_RETURN => {
                        *outcome.borrow_mut() = Some(match pending_match.borrow_mut().take() {
                            Some((service_id, service_name)) => Outcome::Matched { service_id, service_name },
                            None => Outcome::Cancelled,
                        });
                        app.stopModal();
                    }
                    KEYCODE_ESCAPE => {
                        *outcome.borrow_mut() = Some(Outcome::Cancelled);
                        app.stopModal();
                    }
                    // Any other key while awaiting confirmation is ignored
                    // rather than feeding into the (already-finished) combo
                    // capture buffer.
                    _ => {}
                }
                return std::ptr::null_mut();
            }

            match key_code {
                KEYCODE_RETURN => {
                    let recorded_str = tokens.borrow().join(" ");
                    let recorded = Combo::parse(&recorded_str);
                    let gaps = compute_gaps(&timestamps.borrow());
                    match match_combo(recorded, &gaps, &profiles, &registry) {
                        Outcome::Matched { service_id, service_name } if decision == PasteDecision::ConfirmFirst => {
                            eprintln!(
                                "comboauth-daemon: picker matched '{service_name}', focused field is {field_kind:?} -- press Enter again to paste, Esc to cancel"
                            );
                            *pending_match.borrow_mut() = Some((service_id, service_name));
                            *awaiting_confirm.borrow_mut() = true;
                        }
                        other => {
                            *outcome.borrow_mut() = Some(other);
                            app.stopModal();
                        }
                    }
                }
                KEYCODE_ESCAPE => {
                    *outcome.borrow_mut() = Some(Outcome::Cancelled);
                    app.stopModal();
                }
                ARROW_UP => push_token(&tokens, &timestamps, "up"),
                ARROW_DOWN => push_token(&tokens, &timestamps, "down"),
                ARROW_LEFT => push_token(&tokens, &timestamps, "left"),
                ARROW_RIGHT => push_token(&tokens, &timestamps, "right"),
                _ => {
                    if let Some(ch) = event_ref.characters()
                        .map(|s| s.to_string())
                        .and_then(|s| s.chars().next())
                    {
                        push_token(&tokens, &timestamps, &mapped_char_token(ch));
                    }
                }
            }
            std::ptr::null_mut()
        });

        unsafe {
            NSEvent::addLocalMonitorForEventsMatchingMask_handler(NSEventMask::KeyDown, &block)
        }
    };

    app.runModalForWindow(&panel);

    if let Some(monitor) = monitor {
        unsafe { NSEvent::removeMonitor(&monitor) };
    }
    panel.orderOut(None);

    if let Some(prev) = previous_app {
        prev.activateWithOptions(NSApplicationActivationOptions::empty());
    }

    outcome.borrow_mut().take().unwrap_or(Outcome::Cancelled)
}

fn push_token(tokens: &Rc<RefCell<Vec<String>>>, timestamps: &Rc<RefCell<Vec<Instant>>>, token: &str) {
    tokens.borrow_mut().push(token.to_owned());
    timestamps.borrow_mut().push(Instant::now());
}

/// Mirrors `App::record_combo_shortcut`'s char -> token mapping so combos
/// recorded in the TUI match what the picker captures.
fn mapped_char_token(ch: char) -> String {
    match ch.to_ascii_lowercase() {
        'u' => "up".to_owned(),
        'd' => "down".to_owned(),
        'l' => "left".to_owned(),
        'r' => "right".to_owned(),
        'a' => "A".to_owned(),
        'b' => "B".to_owned(),
        'x' => "X".to_owned(),
        'y' => "Y".to_owned(),
        '7' => "up-left".to_owned(),
        '9' => "up-right".to_owned(),
        '1' => "down-left".to_owned(),
        '3' => "down-right".to_owned(),
        other if other.is_ascii_alphabetic() => other.to_ascii_uppercase().to_string(),
        other => other.to_string(),
    }
}

fn compute_gaps(timestamps: &[Instant]) -> Vec<u64> {
    timestamps
        .windows(2)
        .map(|w| w[1].duration_since(w[0]).as_millis() as u64)
        .collect()
}

fn gaps_pass_tolerance(recorded: &[u64], expected: &[u64], tolerance_pct: u32) -> bool {
    if expected.is_empty() {
        return true;
    }
    if recorded.len() != expected.len() {
        return false;
    }
    let tol = tolerance_pct.min(100) as f64 / 100.0;
    recorded.iter().zip(expected.iter()).all(|(&got, &exp)| {
        let lo = (exp as f64 * (1.0 - tol)) as u64;
        let hi = (exp as f64 * (1.0 + tol)) as u64;
        got >= lo && got <= hi
    })
}

fn match_combo(
    recorded: Option<Combo>,
    gaps: &[u64],
    profiles: &[ComboProfile],
    registry: &ServiceRegistry,
) -> Outcome {
    let Some(recorded) = recorded else {
        return Outcome::NoMatch;
    };

    let matched: Option<ComboProfileId> = profiles.iter().find_map(|profile| {
        let expected = Combo::parse(&profile.sequence)?;
        if recorded != expected {
            return None;
        }
        let timing_ok = if profile.gaps_ms.is_empty() {
            true
        } else {
            gaps_pass_tolerance(gaps, &profile.gaps_ms, TIMING_TOLERANCE_PCT)
        };
        timing_ok.then(|| profile.id.clone())
    });

    let Some(combo_profile_id) = matched else {
        return Outcome::NoMatch;
    };

    match registry.service_for_combo_profile(&combo_profile_id) {
        Some(service) => Outcome::Matched {
            service_id: service.id.clone(),
            service_name: service.name.clone(),
        },
        None => Outcome::NoServiceForCombo,
    }
}

fn build_panel(mtm: MainThreadMarker) -> Retained<NSPanel> {
    let screen_frame = objc2_app_kit::NSScreen::mainScreen(mtm)
        .map(|screen| screen.frame())
        .unwrap_or(NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(1440.0, 900.0)));

    let width = 280.0;
    let height = 80.0;
    let origin = NSPoint::new(
        screen_frame.origin.x + (screen_frame.size.width - width) / 2.0,
        screen_frame.origin.y + (screen_frame.size.height - height) / 2.0,
    );
    let content_rect = NSRect::new(origin, NSSize::new(width, height));

    let panel = NSPanel::initWithContentRect_styleMask_backing_defer(
        mtm.alloc(),
        content_rect,
        NSWindowStyleMask::Borderless,
        NSBackingStoreType::Buffered,
        false,
    );
    panel.setLevel(NSFloatingWindowLevel);
    panel.setHasShadow(true);
    panel.setOpaque(false);
    panel.setBackgroundColor(Some(&NSColor::colorWithRed_green_blue_alpha(0.1, 0.1, 0.12, 0.92)));
    panel
}
