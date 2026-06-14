use std::time::{Duration, Instant};

use crate::activation::ActivationResult;
use crate::combo::Combo;
pub use crate::profile::{ComboProfile, ComboProfileId};
use crate::service::{ServiceId, ServiceRecord, ServiceRegistry, ServiceStatus};
use crate::vault::mock::MockSecretStore;
use crate::vault::{SecretMaterial, SecretStore};
use crate::delivery::{ClipboardSink, SecretSink, CLIPBOARD_TIMEOUT_SECS};

pub const UNLOCK_TIMEOUT_SECS: u64 = 15;

#[derive(Debug)]
pub struct App {
    pub should_quit: bool,
    pub current_screen: Screen,
    pub selected_home_item: usize,
    pub selected_detail_item: usize,
    pub home_items: Vec<Screen>,
    pub service_registry: ServiceRegistry,
    pub combo_profiles: Vec<ComboProfile>,
    pub settings: Vec<SettingEntry>,
    pub demo_combo: Option<Combo>,
    pub recorded_combo_tokens: Vec<String>,
    pub recorded_timestamps: Vec<Instant>,
    pub timing_tolerance_pct: u32,
    pub last_activation: ActivationResult,
    pub unlock_time: Option<Instant>,
    pub clipboard_clear_at: Option<Instant>,
    pub quick_launch_open: bool,
    pub quick_launch_tokens: Vec<String>,
    pub quick_launch_timestamps: Vec<Instant>,
    pub record_phase: RecordPhase,
    pub record_name_input: String,
    pub services_phase: ServicesPhase,
    pub service_name_input: String,
    pub services_assign_cursor: usize,
    secret_store: MockSecretStore,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    Home,
    Services,
    Combos,
    TestLab,
    Settings,
    RecordCombo,
    Quit,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecordPhase {
    NameEntry,
    TokenCapture,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ServicesPhase {
    List,
    AddName,
    AssignCombo,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SettingEntry {
    pub name: &'static str,
    pub value: &'static str,
}

impl Screen {
    pub fn label(self) -> &'static str {
        match self {
            Screen::Home => "Home",
            Screen::Services => "Services",
            Screen::Combos => "Combos",
            Screen::TestLab => "Test Lab",
            Screen::Settings => "Settings",
            Screen::RecordCombo => "Record Combo",
            Screen::Quit => "Quit",
        }
    }
}

fn default_service_registry() -> ServiceRegistry {
    ServiceRegistry::new(vec![
        ServiceRecord {
            id: ServiceId("github".to_owned()),
            name: "GitHub".to_owned(),
            username: "demo.dev".to_owned(),
            combo_profile_id: Some(ComboProfileId("quarter-turn".to_owned())),
            pinned: true,
            status: ServiceStatus::Ready,
        },
        ServiceRecord {
            id: ServiceId("wiki".to_owned()),
            name: "Research Wiki".to_owned(),
            username: "astro.local".to_owned(),
            combo_profile_id: Some(ComboProfileId("dash-confirm".to_owned())),
            pinned: false,
            status: ServiceStatus::Ready,
        },
        ServiceRecord {
            id: ServiceId("lab".to_owned()),
            name: "Lab Notes".to_owned(),
            username: "mock-user".to_owned(),
            combo_profile_id: Some(ComboProfileId("focus-reset".to_owned())),
            pinned: false,
            status: ServiceStatus::Ready,
        },
    ])
}

fn default_secret_store() -> MockSecretStore {
    let mut store = MockSecretStore::new();
    store
        .put_secret(
            ServiceId("github".to_owned()),
            SecretMaterial::new(b"***mock-gh-token-abc123***".to_vec()),
        )
        .unwrap();
    store
        .put_secret(
            ServiceId("wiki".to_owned()),
            SecretMaterial::new(b"***mock-wiki-pass-xyz789***".to_vec()),
        )
        .unwrap();
    store
        .put_secret(
            ServiceId("lab".to_owned()),
            SecretMaterial::new(b"***mock-lab-key-def456***".to_vec()),
        )
        .unwrap();
    store
}

impl Default for App {
    fn default() -> Self {
        let mut app = Self {
            should_quit: false,
            current_screen: Screen::Home,
            selected_home_item: 0,
            selected_detail_item: 0,
            home_items: vec![
                Screen::Services,
                Screen::Combos,
                Screen::TestLab,
                Screen::Settings,
                Screen::Quit,
            ],
            service_registry: default_service_registry(),
            combo_profiles: vec![
                ComboProfile {
                    id: ComboProfileId("quarter-turn".to_owned()),
                    name: "Quarter Turn".to_owned(),
                    sequence: "down right A".to_owned(),
                    status: "parsed".to_owned(),
                    timing_window_ms: 300,
                    gaps_ms: vec![],
                },
                ComboProfile {
                    id: ComboProfileId("dash-confirm".to_owned()),
                    name: "Dash Confirm".to_owned(),
                    sequence: "left right B".to_owned(),
                    status: "mock".to_owned(),
                    timing_window_ms: 400,
                    gaps_ms: vec![],
                },
                ComboProfile {
                    id: ComboProfileId("focus-reset".to_owned()),
                    name: "Focus Reset".to_owned(),
                    sequence: "up down X".to_owned(),
                    status: "mock".to_owned(),
                    timing_window_ms: 500,
                    gaps_ms: vec![],
                },
            ],
            settings: vec![
                SettingEntry { name: "Timing Tolerance", value: "40%" },
                SettingEntry { name: "Theme", value: "terminal default" },
                SettingEntry { name: "Secret Handling", value: "disabled" },
            ],
            demo_combo: Combo::parse("down right A"),
            recorded_combo_tokens: Vec::new(),
            recorded_timestamps: Vec::new(),
            timing_tolerance_pct: 40,
            last_activation: ActivationResult::Waiting,
            unlock_time: None,
            clipboard_clear_at: None,
            quick_launch_open: false,
            quick_launch_tokens: Vec::new(),
            quick_launch_timestamps: Vec::new(),
            record_phase: RecordPhase::NameEntry,
            record_name_input: String::new(),
            services_phase: ServicesPhase::List,
            service_name_input: String::new(),
            services_assign_cursor: 0,
            secret_store: default_secret_store(),
        };
        app.sync_service_statuses();
        app
    }
}

impl App {
    pub fn quit(&mut self) {
        self.should_quit = true;
    }

    pub fn activate_selected(&mut self) {
        if self.current_screen != Screen::Home {
            return;
        }
        match self.home_items[self.selected_home_item] {
            Screen::Quit => self.quit(),
            screen => {
                self.current_screen = screen;
                self.selected_detail_item = 0;
            }
        }
    }

    pub fn go_home(&mut self) {
        self.services_phase = ServicesPhase::List;
        self.service_name_input.clear();
        self.lock_on_exit();
        self.current_screen = Screen::Home;
        self.selected_detail_item = 0;
    }

    pub fn next_screen(&mut self) {
        if self.current_screen == Screen::RecordCombo {
            self.cancel_record_combo();
            return;
        }
        self.services_phase = ServicesPhase::List;
        self.service_name_input.clear();
        self.lock_on_exit();
        self.current_screen = match self.current_screen {
            Screen::Home => self.home_items[self.selected_home_item],
            Screen::Services => Screen::Combos,
            Screen::Combos => Screen::TestLab,
            Screen::TestLab => Screen::Settings,
            Screen::Settings => Screen::Services,
            Screen::Quit | Screen::RecordCombo => Screen::Home,
        };
        self.selected_detail_item = 0;
    }

    pub fn previous_screen(&mut self) {
        if self.current_screen == Screen::RecordCombo {
            self.cancel_record_combo();
            return;
        }
        self.services_phase = ServicesPhase::List;
        self.service_name_input.clear();
        self.lock_on_exit();
        self.current_screen = match self.current_screen {
            Screen::Home => self.home_items[self.selected_home_item],
            Screen::Services => Screen::Settings,
            Screen::Combos => Screen::Services,
            Screen::TestLab => Screen::Combos,
            Screen::Settings => Screen::TestLab,
            Screen::Quit | Screen::RecordCombo => Screen::Home,
        };
        self.selected_detail_item = 0;
    }

    pub fn next_item(&mut self) {
        let count = self.item_count();
        if count == 0 { return; }
        let selected = self.selected_index_mut();
        *selected = (*selected + 1) % count;
    }

    pub fn previous_item(&mut self) {
        let count = self.item_count();
        if count == 0 { return; }
        let selected = self.selected_index_mut();
        *selected = if *selected == 0 { count - 1 } else { *selected - 1 };
    }

    pub fn is_test_lab(&self) -> bool {
        self.current_screen == Screen::TestLab
    }

    pub fn record_combo_shortcut(&mut self, key: char) -> bool {
        if !self.can_record_combo_tokens() {
            return false;
        }
        let token = match key.to_ascii_lowercase() {
            'u' => "up",
            'd' => "down",
            'l' => "left",
            'r' => "right",
            'a' => "A",
            'b' => "B",
            'x' => "X",
            'y' => "Y",
            '7' => "up-left",
            '9' => "up-right",
            '1' => "down-left",
            '3' => "down-right",
            other => {
                let s = if other.is_ascii_alphabetic() {
                    other.to_ascii_uppercase().to_string()
                } else {
                    other.to_string()
                };
                self.recorded_combo_tokens.push(s);
                self.recorded_timestamps.push(Instant::now());
                self.last_activation = ActivationResult::Waiting;
                self.unlock_time = None;
                return true;
            }
        };
        self.recorded_combo_tokens.push(token.to_owned());
        self.recorded_timestamps.push(Instant::now());
        self.last_activation = ActivationResult::Waiting;
        self.unlock_time = None;
        true
    }

    pub fn record_combo_token(&mut self, token: &str) {
        if !self.can_record_combo_tokens() { return; }
        self.recorded_combo_tokens.push(token.to_owned());
        self.recorded_timestamps.push(Instant::now());
        self.last_activation = ActivationResult::Waiting;
        self.unlock_time = None;
    }

    pub fn pop_recorded_combo_token(&mut self) {
        if self.can_record_combo_tokens() {
            self.recorded_combo_tokens.pop();
            self.recorded_timestamps.pop();
            self.last_activation = ActivationResult::Waiting;
            self.unlock_time = None;
        }
    }

    pub fn clear_recorded_combo(&mut self) {
        if self.can_record_combo_tokens() {
            self.recorded_combo_tokens.clear();
            self.recorded_timestamps.clear();
            self.last_activation = ActivationResult::Waiting;
            self.unlock_time = None;
        }
    }

    pub fn test_recorded_combo(&mut self) {
        let Some(recorded) = Combo::parse(&self.recorded_combo_input()) else {
            self.recorded_combo_tokens.clear();
            self.recorded_timestamps.clear();
            self.last_activation = ActivationResult::InvalidInput;
            return;
        };

        let test_gaps = self.recorded_gaps_ms();

        let matched = self.combo_profiles.iter().find_map(|profile| {
            let expected = Combo::parse(&profile.sequence)?;
            if recorded != expected { return None; }
            let timing_ok = if profile.gaps_ms.is_empty() {
                true
            } else {
                gaps_pass_tolerance(&test_gaps, &profile.gaps_ms, self.timing_tolerance_pct)
            };
            if timing_ok { Some(profile.id.clone()) } else { None }
        });

        let sequence_matched_any = matched.is_none()
            && self.combo_profiles.iter().any(|p| {
                Combo::parse(&p.sequence).map(|e| recorded == e).unwrap_or(false)
            });

        self.recorded_combo_tokens.clear();
        self.recorded_timestamps.clear();

        if let Some(combo_profile_id) = matched {
            let lookup = self
                .service_registry
                .service_for_combo_profile(&combo_profile_id)
                .map(|s| (s.id.clone(), s.name.clone()));

            if let Some((service_id, service_name)) = lookup {
                if let Ok(secret) = self.secret_store.get_secret(&service_id) {
                    if ClipboardSink.deliver(&secret).is_ok() {
                        self.clipboard_clear_at = Some(
                            Instant::now() + Duration::from_secs(CLIPBOARD_TIMEOUT_SECS),
                        );
                    }
                }
                self.last_activation = ActivationResult::Activated { service_id, service_name };
                self.unlock_time = Some(Instant::now());
            } else {
                let combo_name = self.combo_profiles.iter()
                    .find(|p| p.id == combo_profile_id)
                    .map(|p| p.name.clone())
                    .unwrap_or_default();
                self.last_activation =
                    ActivationResult::NoServiceForCombo { combo_profile_id, combo_name };
                self.unlock_time = None;
            }
        } else if sequence_matched_any {
            self.last_activation = ActivationResult::TimingMismatch;
            self.unlock_time = None;
        } else {
            self.last_activation = ActivationResult::NoMatch;
            self.unlock_time = None;
        }
    }

    /// Attempt activation from the quick-launch overlay. Clears tokens regardless of outcome.
    pub fn activate_quick_launch(&mut self) {
        let input = self.quick_launch_tokens.join(" ");
        self.quick_launch_tokens.clear();
        self.quick_launch_timestamps.clear();
        self.quick_launch_open = false;

        let Some(recorded) = Combo::parse(&input) else {
            self.last_activation = ActivationResult::InvalidInput;
            return;
        };

        let matched = self.combo_profiles.iter().find_map(|profile| {
            let expected = Combo::parse(&profile.sequence)?;
            if recorded == expected { Some(profile.id.clone()) } else { None }
        });

        if let Some(combo_profile_id) = matched {
            let lookup = self
                .service_registry
                .service_for_combo_profile(&combo_profile_id)
                .map(|s| (s.id.clone(), s.name.clone()));

            if let Some((service_id, service_name)) = lookup {
                if let Ok(secret) = self.secret_store.get_secret(&service_id) {
                    if ClipboardSink.deliver(&secret).is_ok() {
                        self.clipboard_clear_at = Some(
                            Instant::now() + Duration::from_secs(CLIPBOARD_TIMEOUT_SECS),
                        );
                    }
                }
                self.last_activation = ActivationResult::Activated { service_id, service_name };
                self.unlock_time = Some(Instant::now());
            } else {
                let combo_name = self.combo_profiles.iter()
                    .find(|p| p.id == combo_profile_id)
                    .map(|p| p.name.clone())
                    .unwrap_or_default();
                self.last_activation =
                    ActivationResult::NoServiceForCombo { combo_profile_id, combo_name };
                self.unlock_time = None;
            }
        } else {
            self.last_activation = ActivationResult::NoMatch;
            self.unlock_time = None;
        }
    }

    pub fn recorded_combo_input(&self) -> String {
        self.recorded_combo_tokens.join(" ")
    }

    pub fn recorded_gaps_ms(&self) -> Vec<u64> {
        self.recorded_timestamps
            .windows(2)
            .map(|w| w[1].duration_since(w[0]).as_millis() as u64)
            .collect()
    }

    pub fn is_record_combo(&self) -> bool {
        self.current_screen == Screen::RecordCombo
    }

    pub fn is_record_combo_name_entry(&self) -> bool {
        self.current_screen == Screen::RecordCombo && self.record_phase == RecordPhase::NameEntry
    }

    pub fn is_record_combo_token_capture(&self) -> bool {
        self.current_screen == Screen::RecordCombo && self.record_phase == RecordPhase::TokenCapture
    }

    pub fn start_record_combo(&mut self) {
        self.current_screen = Screen::RecordCombo;
        self.record_phase = RecordPhase::NameEntry;
        self.record_name_input.clear();
        self.recorded_combo_tokens.clear();
        self.recorded_timestamps.clear();
        self.last_activation = ActivationResult::Waiting;
        self.unlock_time = None;
    }

    pub fn record_name_push_char(&mut self, ch: char) {
        if self.current_screen != Screen::RecordCombo
            || self.record_phase != RecordPhase::NameEntry
        {
            return;
        }
        if self.record_name_input.len() < 40 && ch.is_ascii() && !ch.is_ascii_control() {
            self.record_name_input.push(ch);
        }
    }

    pub fn record_name_backspace(&mut self) {
        if self.current_screen == Screen::RecordCombo
            && self.record_phase == RecordPhase::NameEntry
        {
            self.record_name_input.pop();
        }
    }

    pub fn confirm_name_entry(&mut self) {
        if self.current_screen != Screen::RecordCombo
            || self.record_phase != RecordPhase::NameEntry
        {
            return;
        }
        if !self.record_name_input.trim().is_empty() {
            self.record_phase = RecordPhase::TokenCapture;
            self.recorded_combo_tokens.clear();
            self.recorded_timestamps.clear();
        }
    }

    pub fn save_recorded_combo(&mut self) {
        if self.current_screen != Screen::RecordCombo
            || self.record_phase != RecordPhase::TokenCapture
        {
            return;
        }
        let name = self.record_name_input.trim().to_owned();
        if name.is_empty() || self.recorded_combo_tokens.is_empty() {
            return;
        }
        let sequence = self.recorded_combo_tokens.join(" ");
        let gaps = self.recorded_gaps_ms();
        let id = ComboProfileId(name.to_lowercase().replace(' ', "-"));
        self.combo_profiles.push(ComboProfile {
            id,
            name,
            sequence,
            status: "recorded".to_owned(),
            timing_window_ms: 500,
            gaps_ms: gaps,
        });
        let last = self.combo_profiles.len() - 1;
        self.selected_detail_item = last;
        self.cancel_record_combo_inner(Screen::Combos);
    }

    pub fn cancel_record_combo(&mut self) {
        if self.current_screen == Screen::RecordCombo {
            self.cancel_record_combo_inner(Screen::Combos);
        }
    }

    pub fn is_services_add_name(&self) -> bool {
        self.current_screen == Screen::Services && self.services_phase == ServicesPhase::AddName
    }

    pub fn is_services_assign_combo(&self) -> bool {
        self.current_screen == Screen::Services && self.services_phase == ServicesPhase::AssignCombo
    }

    pub fn start_add_service(&mut self) {
        if self.current_screen != Screen::Services || self.services_phase != ServicesPhase::List {
            return;
        }
        self.services_phase = ServicesPhase::AddName;
        self.service_name_input.clear();
    }

    pub fn service_name_push_char(&mut self, ch: char) {
        if !self.is_services_add_name() { return; }
        if self.service_name_input.len() < 40 && ch.is_ascii() && !ch.is_ascii_control() {
            self.service_name_input.push(ch);
        }
    }

    pub fn service_name_backspace(&mut self) {
        if self.is_services_add_name() {
            self.service_name_input.pop();
        }
    }

    pub fn save_new_service(&mut self) {
        if !self.is_services_add_name() { return; }
        let name = self.service_name_input.trim().to_owned();
        if name.is_empty() { return; }
        let id = ServiceId(name.to_lowercase().replace(' ', "-"));
        let record = ServiceRecord {
            id,
            name,
            username: String::new(),
            combo_profile_id: None,
            pinned: false,
            status: ServiceStatus::Unassigned,
        };
        if self.service_registry.add(record).is_ok() {
            self.selected_detail_item = self.service_registry.services().len() - 1;
        }
        self.sync_service_statuses();
        self.services_phase = ServicesPhase::List;
        self.service_name_input.clear();
    }

    pub fn start_assign_combo(&mut self) {
        if self.current_screen != Screen::Services
            || self.services_phase != ServicesPhase::List
            || self.combo_profiles.is_empty()
            || self.service_registry.services().is_empty()
        {
            return;
        }
        self.services_phase = ServicesPhase::AssignCombo;
        self.services_assign_cursor = 0;
    }

    pub fn confirm_assign_combo(&mut self) {
        if !self.is_services_assign_combo() { return; }
        if self.combo_profiles.is_empty() || self.service_registry.services().is_empty() { return; }
        let combo_profile_id = self.combo_profiles[self.services_assign_cursor].id.clone();
        if let Some(svc) = self.service_registry.services().get(self.selected_detail_item) {
            let service_id = svc.id.clone();
            let _ = self.service_registry.assign_combo(&service_id, combo_profile_id);
        }
        self.sync_service_statuses();
        self.services_phase = ServicesPhase::List;
    }

    pub fn cancel_services_action(&mut self) {
        if self.current_screen == Screen::Services && self.services_phase != ServicesPhase::List {
            self.services_phase = ServicesPhase::List;
            self.service_name_input.clear();
        }
    }

    pub fn tick(&mut self) {
        if let Some(t) = self.unlock_time {
            if t.elapsed() >= Duration::from_secs(UNLOCK_TIMEOUT_SECS) {
                self.last_activation = ActivationResult::Waiting;
                self.unlock_time = None;
            }
        }
        if let Some(clear_at) = self.clipboard_clear_at {
            if Instant::now() >= clear_at {
                crate::delivery::schedule_clipboard_clear(0);
                self.clipboard_clear_at = None;
            }
        }
    }

    /// Recompute each service's status from the secret store.
    ///
    /// Call after any operation that may change which secrets exist or which
    /// combo profile is assigned to a service.
    pub fn sync_service_statuses(&mut self) {
        let ids_and_combos: Vec<(ServiceId, bool)> = self
            .service_registry
            .services()
            .iter()
            .map(|s| (s.id.clone(), s.combo_profile_id.is_some()))
            .collect();
        for (id, has_combo) in ids_and_combos {
            let status = if !has_combo {
                ServiceStatus::Unassigned
            } else if self.secret_store.contains_secret(&id) {
                ServiceStatus::Ready
            } else {
                ServiceStatus::MissingSecret
            };
            if let Some(svc) = self.service_registry.get_mut(&id) {
                svc.status = status;
            }
        }
    }

    /// Seconds until the clipboard will be cleared, or None if not active.
    pub fn clipboard_secs_remaining(&self) -> Option<u64> {
        self.clipboard_clear_at?
            .checked_duration_since(Instant::now())
            .map(|d| d.as_secs() + 1)
    }

    fn cancel_record_combo_inner(&mut self, destination: Screen) {
        self.current_screen = destination;
        self.record_phase = RecordPhase::NameEntry;
        self.record_name_input.clear();
        self.recorded_combo_tokens.clear();
        self.recorded_timestamps.clear();
        self.last_activation = ActivationResult::Waiting;
        self.unlock_time = None;
    }

    fn lock_on_exit(&mut self) {
        self.last_activation = ActivationResult::Waiting;
        self.unlock_time = None;
    }

    fn can_record_combo_tokens(&self) -> bool {
        self.current_screen == Screen::TestLab
            || (self.current_screen == Screen::RecordCombo
                && self.record_phase == RecordPhase::TokenCapture)
    }

    fn selected_index_mut(&mut self) -> &mut usize {
        if self.current_screen == Screen::Home {
            &mut self.selected_home_item
        } else if self.current_screen == Screen::Services
            && self.services_phase == ServicesPhase::AssignCombo
        {
            &mut self.services_assign_cursor
        } else {
            &mut self.selected_detail_item
        }
    }

    fn item_count(&self) -> usize {
        match self.current_screen {
            Screen::Home => self.home_items.len(),
            Screen::Services => match self.services_phase {
                ServicesPhase::AssignCombo => self.combo_profiles.len(),
                _ => self.service_registry.services().len(),
            },
            Screen::Combos => self.combo_profiles.len(),
            Screen::TestLab => 0,
            Screen::Settings => self.settings.len(),
            Screen::RecordCombo | Screen::Quit => 0,
        }
    }
}

pub(crate) fn gaps_pass_tolerance(recorded: &[u64], expected: &[u64], tolerance_pct: u32) -> bool {
    if expected.is_empty() { return true; }
    if recorded.len() != expected.len() { return false; }
    let tol = tolerance_pct as f64 / 100.0;
    recorded.iter().zip(expected.iter()).all(|(&got, &exp)| {
        let lo = (exp as f64 * (1.0 - tol)) as u64;
        let hi = (exp as f64 * (1.0 + tol)) as u64;
        got >= lo && got <= hi
    })
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use super::{gaps_pass_tolerance, App, RecordPhase, Screen, ServicesPhase, UNLOCK_TIMEOUT_SECS};
    use crate::activation::ActivationResult;
    use crate::profile::{ComboProfile, ComboProfileId};
    use crate::service::{ServiceId, ServiceStatus};

    // --- navigation ---

    #[test]
    fn enter_opens_selected_home_screen() {
        let mut app = App::default();
        app.activate_selected();
        assert_eq!(app.current_screen, Screen::Services);
    }

    #[test]
    fn left_and_right_cycle_detail_screens() {
        let mut app = App::default();
        app.activate_selected();
        app.next_screen();
        assert_eq!(app.current_screen, Screen::Combos);
        app.previous_screen();
        assert_eq!(app.current_screen, Screen::Services);
    }

    #[test]
    fn home_navigation_tracks_home_selection() {
        let mut app = App::default();
        app.next_item();
        assert_eq!(app.home_items[app.selected_home_item], Screen::Combos);
        app.previous_item();
        assert_eq!(app.home_items[app.selected_home_item], Screen::Services);
    }

    // --- combo matching ---

    #[test]
    fn records_and_tests_matching_combo() {
        let mut app = App::default();
        app.current_screen = Screen::TestLab;
        assert!(app.record_combo_shortcut('d'));
        assert!(app.record_combo_shortcut('r'));
        assert!(app.record_combo_shortcut('a'));
        app.test_recorded_combo();
        assert_eq!(app.recorded_combo_input(), "");
        assert!(
            matches!(&app.last_activation, ActivationResult::Activated { service_name, .. } if service_name == "GitHub")
        );
    }

    #[test]
    fn reports_non_matching_combo() {
        let mut app = App::default();
        app.current_screen = Screen::TestLab;
        assert!(app.record_combo_shortcut('u'));
        assert!(app.record_combo_shortcut('x'));
        app.test_recorded_combo();
        assert_eq!(app.last_activation, ActivationResult::NoMatch);
    }

    #[test]
    fn correct_combo_activates_matching_service() {
        let mut app = App::default();
        app.current_screen = Screen::TestLab;
        assert!(app.record_combo_shortcut('d'));
        assert!(app.record_combo_shortcut('r'));
        assert!(app.record_combo_shortcut('a'));
        app.test_recorded_combo();
        assert_eq!(
            app.last_activation,
            ActivationResult::Activated {
                service_id: ServiceId("github".to_owned()),
                service_name: "GitHub".to_owned(),
            }
        );
    }

    #[test]
    fn wrong_combo_leaves_activation_as_no_match() {
        let mut app = App::default();
        app.current_screen = Screen::TestLab;
        assert!(app.record_combo_shortcut('u'));
        assert!(app.record_combo_shortcut('x'));
        app.test_recorded_combo();
        assert_eq!(app.last_activation, ActivationResult::NoMatch);
    }

    #[test]
    fn clear_resets_activation_to_waiting() {
        let mut app = App::default();
        app.current_screen = Screen::TestLab;
        assert!(app.record_combo_shortcut('d'));
        assert!(app.record_combo_shortcut('r'));
        assert!(app.record_combo_shortcut('a'));
        app.test_recorded_combo();
        assert!(matches!(app.last_activation, ActivationResult::Activated { .. }));
        app.clear_recorded_combo();
        assert_eq!(app.last_activation, ActivationResult::Waiting);
    }

    #[test]
    fn records_diagonal_shortcut_tokens() {
        let mut app = App::default();
        app.current_screen = Screen::TestLab;
        assert!(app.record_combo_shortcut('7'));
        assert!(app.record_combo_shortcut('9'));
        assert!(app.record_combo_shortcut('1'));
        assert!(app.record_combo_shortcut('3'));
        assert_eq!(
            app.recorded_combo_input(),
            "up-left up-right down-left down-right"
        );
    }

    #[test]
    fn diagonal_shortcut_parses_as_valid_combo() {
        let mut app = App::default();
        app.current_screen = Screen::TestLab;
        assert!(app.record_combo_shortcut('d'));
        assert!(app.record_combo_shortcut('3'));
        assert!(app.record_combo_shortcut('a'));
        use crate::combo::Combo;
        let parsed = Combo::parse(&app.recorded_combo_input());
        assert!(parsed.is_some(), "diagonal combo should parse");
        assert_eq!(parsed.unwrap().len(), 3);
    }

    #[test]
    fn unknown_shortcut_key_recorded_as_is() {
        let mut app = App::default();
        app.current_screen = Screen::TestLab;
        assert!(app.record_combo_shortcut('z'));
        assert_eq!(app.recorded_combo_input(), "Z");
        assert!(app.record_combo_shortcut('0'));
        assert_eq!(app.recorded_combo_input(), "Z 0");
    }

    #[test]
    fn second_service_combo_activates_correct_service() {
        let mut app = App::default();
        app.current_screen = Screen::TestLab;
        assert!(app.record_combo_shortcut('l'));
        assert!(app.record_combo_shortcut('r'));
        assert!(app.record_combo_shortcut('b'));
        app.test_recorded_combo();
        assert_eq!(
            app.last_activation,
            ActivationResult::Activated {
                service_id: ServiceId("wiki".to_owned()),
                service_name: "Research Wiki".to_owned(),
            }
        );
    }

    #[test]
    fn activation_clears_on_go_home() {
        let mut app = App::default();
        app.current_screen = Screen::TestLab;
        app.record_combo_shortcut('d');
        app.record_combo_shortcut('r');
        app.record_combo_shortcut('a');
        app.test_recorded_combo();
        assert!(matches!(app.last_activation, ActivationResult::Activated { .. }));
        app.go_home();
        assert_eq!(app.last_activation, ActivationResult::Waiting);
    }

    #[test]
    fn activation_clears_on_next_screen() {
        let mut app = App::default();
        app.current_screen = Screen::TestLab;
        app.record_combo_shortcut('d');
        app.record_combo_shortcut('r');
        app.record_combo_shortcut('a');
        app.test_recorded_combo();
        assert!(matches!(app.last_activation, ActivationResult::Activated { .. }));
        app.next_screen();
        assert_eq!(app.last_activation, ActivationResult::Waiting);
    }

    #[test]
    fn activation_clears_on_previous_screen() {
        let mut app = App::default();
        app.current_screen = Screen::TestLab;
        app.record_combo_shortcut('d');
        app.record_combo_shortcut('r');
        app.record_combo_shortcut('a');
        app.test_recorded_combo();
        assert!(matches!(app.last_activation, ActivationResult::Activated { .. }));
        app.previous_screen();
        assert_eq!(app.last_activation, ActivationResult::Waiting);
    }

    // --- timestamps ---

    #[test]
    fn record_shortcut_captures_timestamps() {
        let mut app = App::default();
        app.current_screen = Screen::TestLab;
        app.record_combo_shortcut('d');
        app.record_combo_shortcut('r');
        app.record_combo_shortcut('a');
        assert_eq!(app.recorded_timestamps.len(), 3);
    }

    #[test]
    fn pop_removes_timestamp() {
        let mut app = App::default();
        app.current_screen = Screen::TestLab;
        app.record_combo_shortcut('d');
        app.record_combo_shortcut('r');
        app.pop_recorded_combo_token();
        assert_eq!(app.recorded_timestamps.len(), 1);
        assert_eq!(app.recorded_combo_tokens.len(), 1);
    }

    #[test]
    fn clear_removes_all_timestamps() {
        let mut app = App::default();
        app.current_screen = Screen::TestLab;
        app.record_combo_shortcut('d');
        app.record_combo_shortcut('r');
        app.clear_recorded_combo();
        assert!(app.recorded_timestamps.is_empty());
    }

    #[test]
    fn test_clears_timestamps_on_completion() {
        let mut app = App::default();
        app.current_screen = Screen::TestLab;
        app.record_combo_shortcut('d');
        app.record_combo_shortcut('r');
        app.record_combo_shortcut('a');
        app.test_recorded_combo();
        assert!(app.recorded_timestamps.is_empty());
    }

    #[test]
    fn recorded_gaps_ms_returns_empty_for_single_token() {
        let mut app = App::default();
        app.current_screen = Screen::TestLab;
        app.record_combo_shortcut('d');
        assert!(app.recorded_gaps_ms().is_empty());
    }

    #[test]
    fn recorded_gaps_ms_count_matches_token_pairs() {
        let mut app = App::default();
        app.current_screen = Screen::TestLab;
        app.record_combo_shortcut('d');
        app.record_combo_shortcut('r');
        app.record_combo_shortcut('a');
        assert_eq!(app.recorded_gaps_ms().len(), 2);
    }

    // --- timing with tolerance ---

    fn make_app_with_timed_profile(gaps_ms: Vec<u64>) -> App {
        let mut app = App::default();
        app.current_screen = Screen::TestLab;
        app.combo_profiles[0] = ComboProfile {
            id: ComboProfileId("quarter-turn".to_owned()),
            name: "Quarter Turn".to_owned(),
            sequence: "down right A".to_owned(),
            status: "parsed".to_owned(),
            timing_window_ms: 300,
            gaps_ms,
        };
        app
    }

    #[test]
    fn timing_match_passes_when_profile_has_no_gaps() {
        let mut app = App::default();
        app.current_screen = Screen::TestLab;
        app.record_combo_shortcut('d');
        app.record_combo_shortcut('r');
        app.record_combo_shortcut('a');
        app.test_recorded_combo();
        assert!(matches!(app.last_activation, ActivationResult::Activated { .. }));
    }

    #[test]
    fn timing_mismatch_when_gaps_outside_tolerance() {
        let mut app = make_app_with_timed_profile(vec![200, 200]);
        let t0 = Instant::now();
        app.record_combo_token("down");
        app.record_combo_token("right");
        app.record_combo_token("A");
        app.recorded_timestamps = vec![
            t0,
            t0 + Duration::from_millis(500),
            t0 + Duration::from_millis(1000),
        ];
        app.test_recorded_combo();
        assert_eq!(app.last_activation, ActivationResult::TimingMismatch);
    }

    #[test]
    fn timing_match_passes_within_tolerance() {
        let mut app = make_app_with_timed_profile(vec![200, 200]);
        let t0 = Instant::now();
        app.record_combo_token("down");
        app.record_combo_token("right");
        app.record_combo_token("A");
        app.recorded_timestamps = vec![
            t0,
            t0 + Duration::from_millis(210),
            t0 + Duration::from_millis(430),
        ];
        app.test_recorded_combo();
        assert!(
            matches!(&app.last_activation, ActivationResult::Activated { service_name, .. } if service_name == "GitHub")
        );
    }

    // --- gaps_pass_tolerance ---

    #[test]
    fn tolerance_empty_expected_always_passes() {
        assert!(gaps_pass_tolerance(&[100, 200], &[], 40));
        assert!(gaps_pass_tolerance(&[], &[], 40));
    }

    #[test]
    fn tolerance_exact_match_passes() {
        assert!(gaps_pass_tolerance(&[200, 150], &[200, 150], 40));
    }

    #[test]
    fn tolerance_within_band_passes() {
        assert!(gaps_pass_tolerance(&[250], &[200], 40));
        assert!(gaps_pass_tolerance(&[130], &[200], 40));
    }

    #[test]
    fn tolerance_outside_band_fails() {
        assert!(!gaps_pass_tolerance(&[300], &[200], 40));
        assert!(!gaps_pass_tolerance(&[100], &[200], 40));
    }

    #[test]
    fn tolerance_length_mismatch_fails() {
        assert!(!gaps_pass_tolerance(&[100], &[100, 200], 40));
        assert!(!gaps_pass_tolerance(&[100, 200], &[100], 40));
    }

    #[test]
    fn tolerance_zero_pct_requires_exact() {
        assert!(gaps_pass_tolerance(&[200], &[200], 0));
        assert!(!gaps_pass_tolerance(&[201], &[200], 0));
    }

    // --- combo recording flow ---

    #[test]
    fn start_record_combo_enters_name_entry() {
        let mut app = App::default();
        app.start_record_combo();
        assert_eq!(app.current_screen, Screen::RecordCombo);
        assert_eq!(app.record_phase, RecordPhase::NameEntry);
        assert!(app.record_name_input.is_empty());
    }

    #[test]
    fn record_name_push_char_appends_to_input() {
        let mut app = App::default();
        app.start_record_combo();
        app.record_name_push_char('H');
        app.record_name_push_char('i');
        assert_eq!(app.record_name_input, "Hi");
    }

    #[test]
    fn record_name_backspace_removes_last_char() {
        let mut app = App::default();
        app.start_record_combo();
        app.record_name_push_char('A');
        app.record_name_push_char('B');
        app.record_name_backspace();
        assert_eq!(app.record_name_input, "A");
    }

    #[test]
    fn confirm_name_entry_transitions_to_token_capture() {
        let mut app = App::default();
        app.start_record_combo();
        app.record_name_push_char('T');
        app.confirm_name_entry();
        assert_eq!(app.record_phase, RecordPhase::TokenCapture);
    }

    #[test]
    fn confirm_name_entry_rejected_when_name_is_blank() {
        let mut app = App::default();
        app.start_record_combo();
        app.confirm_name_entry();
        assert_eq!(app.record_phase, RecordPhase::NameEntry);
    }

    #[test]
    fn token_capture_accepts_combo_shortcuts() {
        let mut app = App::default();
        app.start_record_combo();
        app.record_name_push_char('T');
        app.confirm_name_entry();
        assert!(app.record_combo_shortcut('d'));
        assert!(app.record_combo_shortcut('r'));
        assert!(app.record_combo_shortcut('a'));
        assert_eq!(app.recorded_combo_input(), "down right A");
    }

    #[test]
    fn shortcuts_rejected_in_name_entry_phase() {
        let mut app = App::default();
        app.start_record_combo();
        assert!(!app.record_combo_shortcut('d'));
        assert!(app.recorded_combo_input().is_empty());
    }

    #[test]
    fn save_recorded_combo_adds_profile_and_goes_to_combos() {
        let initial_count = App::default().combo_profiles.len();
        let mut app = App::default();
        app.start_record_combo();
        app.record_name_push_char('M');
        app.record_name_push_char('y');
        app.confirm_name_entry();
        app.record_combo_shortcut('u');
        app.record_combo_shortcut('d');
        app.record_combo_shortcut('a');
        app.save_recorded_combo();
        assert_eq!(app.combo_profiles.len(), initial_count + 1);
        assert_eq!(app.current_screen, Screen::Combos);
        let saved = app.combo_profiles.last().unwrap();
        assert_eq!(saved.name, "My");
        assert_eq!(saved.sequence, "up down A");
        assert_eq!(saved.status, "recorded");
    }

    #[test]
    fn save_recorded_combo_captures_gaps() {
        let mut app = App::default();
        app.start_record_combo();
        app.record_name_push_char('G');
        app.confirm_name_entry();
        app.record_combo_shortcut('d');
        app.record_combo_shortcut('r');
        app.record_combo_shortcut('a');
        app.save_recorded_combo();
        let saved = app.combo_profiles.last().unwrap();
        assert_eq!(saved.gaps_ms.len(), 2);
    }

    #[test]
    fn save_noop_when_tokens_empty() {
        let initial_count = App::default().combo_profiles.len();
        let mut app = App::default();
        app.start_record_combo();
        app.record_name_push_char('T');
        app.confirm_name_entry();
        app.save_recorded_combo();
        assert_eq!(app.combo_profiles.len(), initial_count);
        assert_eq!(app.current_screen, Screen::RecordCombo);
    }

    #[test]
    fn cancel_record_combo_returns_to_combos() {
        let mut app = App::default();
        app.start_record_combo();
        app.record_name_push_char('X');
        app.cancel_record_combo();
        assert_eq!(app.current_screen, Screen::Combos);
        assert!(app.record_name_input.is_empty());
    }

    // --- Services flow ---

    #[test]
    fn start_add_service_enters_add_name_phase() {
        let mut app = App::default();
        app.current_screen = Screen::Services;
        app.start_add_service();
        assert_eq!(app.services_phase, ServicesPhase::AddName);
        assert!(app.service_name_input.is_empty());
    }

    #[test]
    fn save_new_service_adds_entry_and_returns_to_list() {
        let initial = App::default().service_registry.services().len();
        let mut app = App::default();
        app.current_screen = Screen::Services;
        app.start_add_service();
        app.service_name_push_char('M');
        app.service_name_push_char('y');
        app.service_name_push_char('S');
        app.service_name_push_char('v');
        app.service_name_push_char('c');
        app.save_new_service();
        assert_eq!(app.service_registry.services().len(), initial + 1);
        assert_eq!(app.service_registry.services().last().unwrap().name, "MySvc");
        assert_eq!(app.services_phase, ServicesPhase::List);
        assert_eq!(app.selected_detail_item, initial);
    }

    #[test]
    fn save_new_service_noop_when_name_blank() {
        let initial = App::default().service_registry.services().len();
        let mut app = App::default();
        app.current_screen = Screen::Services;
        app.start_add_service();
        app.save_new_service();
        assert_eq!(app.service_registry.services().len(), initial);
        assert_eq!(app.services_phase, ServicesPhase::AddName);
    }

    #[test]
    fn cancel_services_action_returns_to_list() {
        let mut app = App::default();
        app.current_screen = Screen::Services;
        app.start_add_service();
        app.service_name_push_char('X');
        app.cancel_services_action();
        assert_eq!(app.services_phase, ServicesPhase::List);
        assert!(app.service_name_input.is_empty());
    }

    #[test]
    fn start_assign_combo_enters_assign_phase() {
        let mut app = App::default();
        app.current_screen = Screen::Services;
        app.start_assign_combo();
        assert_eq!(app.services_phase, ServicesPhase::AssignCombo);
        assert_eq!(app.services_assign_cursor, 0);
    }

    #[test]
    fn confirm_assign_combo_sets_combo_profile_on_service() {
        let mut app = App::default();
        app.current_screen = Screen::Services;
        app.selected_detail_item = 0;
        app.start_assign_combo();
        app.services_assign_cursor = 0;
        app.confirm_assign_combo();
        assert_eq!(
            app.service_registry.services()[0].combo_profile_id,
            Some(ComboProfileId("quarter-turn".to_owned()))
        );
        assert_eq!(app.services_phase, ServicesPhase::List);
    }

    #[test]
    fn assign_combo_cursor_navigates_profile_list() {
        let mut app = App::default();
        app.current_screen = Screen::Services;
        app.start_assign_combo();
        app.next_item();
        assert_eq!(app.services_assign_cursor, 1);
    }

    #[test]
    fn start_assign_combo_noop_when_no_profiles() {
        let mut app = App::default();
        app.current_screen = Screen::Services;
        app.combo_profiles.clear();
        app.start_assign_combo();
        assert_eq!(app.services_phase, ServicesPhase::List);
    }

    #[test]
    fn service_name_max_length_enforced() {
        let mut app = App::default();
        app.current_screen = Screen::Services;
        app.start_add_service();
        for _ in 0..50 { app.service_name_push_char('a'); }
        assert_eq!(app.service_name_input.len(), 40);
    }

    #[test]
    fn record_combo_name_max_length_enforced() {
        let mut app = App::default();
        app.start_record_combo();
        for _ in 0..50 { app.record_name_push_char('a'); }
        assert_eq!(app.record_name_input.len(), 40);
    }

    // --- unlock timeout ---

    #[test]
    fn activation_relocks_after_timeout() {
        let mut app = App::default();
        app.current_screen = Screen::TestLab;
        app.record_combo_shortcut('d');
        app.record_combo_shortcut('r');
        app.record_combo_shortcut('a');
        app.test_recorded_combo();
        assert!(matches!(app.last_activation, ActivationResult::Activated { .. }));
        assert!(app.unlock_time.is_some());
        app.unlock_time = Some(Instant::now() - Duration::from_secs(UNLOCK_TIMEOUT_SECS + 1));
        app.tick();
        assert_eq!(app.last_activation, ActivationResult::Waiting);
        assert!(app.unlock_time.is_none());
    }

    #[test]
    fn activation_stays_within_timeout() {
        let mut app = App::default();
        app.current_screen = Screen::TestLab;
        app.record_combo_shortcut('d');
        app.record_combo_shortcut('r');
        app.record_combo_shortcut('a');
        app.test_recorded_combo();
        assert!(matches!(app.last_activation, ActivationResult::Activated { .. }));
        app.tick();
        assert!(matches!(app.last_activation, ActivationResult::Activated { .. }));
    }

    #[test]
    fn unlock_time_set_on_successful_match() {
        let mut app = App::default();
        app.current_screen = Screen::TestLab;
        app.record_combo_shortcut('d');
        app.record_combo_shortcut('r');
        app.record_combo_shortcut('a');
        app.test_recorded_combo();
        assert!(app.unlock_time.is_some());
    }

    #[test]
    fn unlock_time_none_on_failed_match() {
        let mut app = App::default();
        app.current_screen = Screen::TestLab;
        app.record_combo_shortcut('u');
        app.record_combo_shortcut('x');
        app.test_recorded_combo();
        assert!(app.unlock_time.is_none());
    }

    #[test]
    fn unlock_time_cleared_on_go_home() {
        let mut app = App::default();
        app.current_screen = Screen::TestLab;
        app.record_combo_shortcut('d');
        app.record_combo_shortcut('r');
        app.record_combo_shortcut('a');
        app.test_recorded_combo();
        assert!(app.unlock_time.is_some());
        app.go_home();
        assert!(app.unlock_time.is_none());
    }

    #[test]
    fn unlock_time_cleared_on_screen_change() {
        let mut app = App::default();
        app.current_screen = Screen::TestLab;
        app.record_combo_shortcut('d');
        app.record_combo_shortcut('r');
        app.record_combo_shortcut('a');
        app.test_recorded_combo();
        assert!(app.unlock_time.is_some());
        app.next_screen();
        assert!(app.unlock_time.is_none());
    }

    // --- service status sync ---

    #[test]
    fn default_services_with_secrets_are_ready() {
        let app = App::default();
        for svc in app.service_registry.services() {
            // all demo services have secrets and combo profiles pre-assigned
            assert_eq!(svc.status, ServiceStatus::Ready, "{} should be Ready", svc.id.0);
        }
    }

    #[test]
    fn new_service_has_unassigned_status() {
        let mut app = App::default();
        app.current_screen = Screen::Services;
        app.start_add_service();
        for ch in "NewSvc".chars() { app.service_name_push_char(ch); }
        app.save_new_service();
        let svc = app.service_registry.services().last().unwrap();
        assert_eq!(svc.status, ServiceStatus::Unassigned);
    }

    #[test]
    fn assigning_combo_without_secret_marks_missing_secret() {
        let mut app = App::default();
        app.current_screen = Screen::Services;
        // Add a fresh service with no secret in the store
        app.start_add_service();
        for ch in "Orphan".chars() { app.service_name_push_char(ch); }
        app.save_new_service();
        let idx = app.service_registry.services().len() - 1;
        let orphan_id = app.service_registry.services()[idx].id.clone();
        // Assign via registry directly (all default profiles are taken)
        let fresh_combo = ComboProfileId("orphan-combo".to_owned());
        app.service_registry.assign_combo(&orphan_id, fresh_combo).unwrap();
        app.sync_service_statuses();
        assert_eq!(
            app.service_registry.services()[idx].status,
            ServiceStatus::MissingSecret
        );
    }

    #[test]
    fn sync_flips_ready_to_missing_when_secret_removed() {
        use crate::vault::SecretStore;
        let mut app = App::default();
        // GitHub is Ready initially
        let gh_id = crate::service::ServiceId("github".to_owned());
        assert_eq!(
            app.service_registry.get(&gh_id).unwrap().status,
            ServiceStatus::Ready
        );
        // Remove its secret and resync
        app.secret_store.delete_secret(&gh_id).unwrap();
        app.sync_service_statuses();
        assert_eq!(
            app.service_registry.get(&gh_id).unwrap().status,
            ServiceStatus::MissingSecret
        );
    }

    // --- quick launch ---

    #[test]
    fn quick_launch_tokens_cleared_after_activation() {
        let mut app = App::default();
        app.quick_launch_open = true;
        app.quick_launch_tokens.push("down".to_owned());
        app.quick_launch_timestamps.push(Instant::now());
        app.quick_launch_tokens.push("right".to_owned());
        app.quick_launch_timestamps.push(Instant::now());
        app.quick_launch_tokens.push("A".to_owned());
        app.quick_launch_timestamps.push(Instant::now());

        app.activate_quick_launch();

        assert!(app.quick_launch_tokens.is_empty());
        assert!(app.quick_launch_timestamps.is_empty());
        assert!(!app.quick_launch_open);
        assert_eq!(
            app.last_activation,
            ActivationResult::Activated {
                service_id: ServiceId("github".to_owned()),
                service_name: "GitHub".to_owned(),
            }
        );
    }
}
