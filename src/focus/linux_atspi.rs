// Phase 9-D: query focused element via atspi crate.
// Detect Role::PasswordText; return Unknown aggressively when D-Bus unavailable.
//
// Walks the AT-SPI accessibility tree (desktop → apps → windows → widgets,
// max 5 levels deep, max 200 nodes) looking for the first element with
// State::Focused, then maps its Role to a FieldKind. Any D-Bus or AT-SPI
// error returns Unknown — the "aggressive Unknown" policy. Role is advisory
// only; a spoofing app can lie about Role::PasswordText, so Secure only
// reduces accidental-paste risk, not adversarial app defense.

use std::collections::VecDeque;

use atspi::proxy::accessible::ObjectRefExt as _;
use atspi::{AccessibilityConnection, ObjectRef, Role, State};

const MAX_NODES: usize = 200;
const CONNECT_MS: u64 = 500;
const CHILDREN_MS: u64 = 200;
const STATE_MS: u64 = 100;
const ROLE_MS: u64 = 100;
const GLOBAL_MS: u64 = 1500;

async fn query_focused_role() -> Option<Role> {
    let conn = tokio::time::timeout(
        std::time::Duration::from_millis(CONNECT_MS),
        AccessibilityConnection::new(),
    )
    .await
    .ok()?
    .ok()?;

    let bus = conn.connection();

    // The AT-SPI registry root is the "desktop" — its children are registered apps.
    let desktop = atspi::proxy::accessible::AccessibleProxy::builder(bus)
        .destination("org.a11y.atspi.Registry")
        .ok()?
        .path("/org/a11y/atspi/accessible/root")
        .ok()?
        .cache_properties(atspi::zbus::proxy::CacheProperties::No)
        .build()
        .await
        .ok()?;

    let apps = tokio::time::timeout(
        std::time::Duration::from_millis(CHILDREN_MS),
        desktop.get_children(),
    )
    .await
    .ok()?
    .ok()?;

    let mut queue: VecDeque<(ObjectRef, u8)> = VecDeque::new();
    for app_ref in apps {
        queue.push_back((app_ref, 0));
    }

    let mut visited = 0usize;

    while let Some((obj_ref, depth)) = queue.pop_front() {
        if visited >= MAX_NODES {
            break;
        }
        visited += 1;

        let proxy = match obj_ref.as_accessible_proxy(bus).await {
            Ok(p) => p,
            Err(_) => return None,
        };

        let state_set = match tokio::time::timeout(
            std::time::Duration::from_millis(STATE_MS),
            proxy.get_state(),
        )
        .await
        {
            Ok(Ok(s)) => s,
            _ => return None,
        };

        if state_set.contains(State::Focused) {
            return match tokio::time::timeout(
                std::time::Duration::from_millis(ROLE_MS),
                proxy.get_role(),
            )
            .await
            {
                Ok(Ok(role)) => Some(role),
                _ => None,
            };
        }

        if depth < 5 {
            match tokio::time::timeout(
                std::time::Duration::from_millis(CHILDREN_MS),
                proxy.get_children(),
            )
            .await
            {
                Ok(Ok(children)) => {
                    for child in children {
                        queue.push_back((child, depth + 1));
                    }
                }
                _ => return None,
            }
        }
    }

    None
}

fn role_to_field_kind(role: Option<Role>) -> super::FieldKind {
    use super::FieldKind;
    match role {
        Some(Role::PasswordText) => FieldKind::Secure,
        Some(Role::Text | Role::Entry | Role::DocumentFrame | Role::Terminal) => FieldKind::Editable,
        Some(
            Role::Button
            | Role::CheckBox
            | Role::RadioButton
            | Role::MenuItem
            | Role::Label
            | Role::Icon
            | Role::Separator,
        ) => FieldKind::NonEditable,
        None | Some(_) => FieldKind::Unknown,
    }
}

/// Async variant — use this inside async Tokio tasks (e.g. IPC handler).
pub async fn focused_field_kind_async() -> super::FieldKind {
    let result = tokio::time::timeout(
        std::time::Duration::from_millis(GLOBAL_MS),
        query_focused_role(),
    )
    .await
    .unwrap_or(None);
    role_to_field_kind(result)
}

/// Sync variant — only call from non-async threads (e.g. the hotkey listener).
/// Panics if called from inside a running Tokio runtime.
pub fn focused_field_kind() -> super::FieldKind {
    let rt = match tokio::runtime::Runtime::new() {
        Ok(rt) => rt,
        Err(_) => return super::FieldKind::Unknown,
    };
    rt.block_on(focused_field_kind_async())
}
