use std::time::Instant;

use crate::combo::Combo;
pub use crate::profile::ComboProfile;

#[derive(Debug, Clone)]
pub struct App {
    pub should_quit: bool,
    pub current_screen: Screen,
    pub selected_home_item: usize,
    pub selected_detail_item: usize,
    pub home_items: Vec<Screen>,
    pub services: Vec<ServiceEntry>,
    pub combo_profiles: Vec<ComboProfile>,
    pub settings: Vec<SettingEntry>,
    pub demo_combo: Option<Combo>,
    pub recorded_combo_tokens: Vec<String>,
    pub recorded_timestamps: Vec<Instant>,
    pub timing_tolerance_pct: u32,
    pub test_result: ComboTestResult,
    pub vault_state: VaultState,
    /// Phase within the RecordCombo screen.
    pub record_phase: RecordPhase,
    /// Text input buffer for the combo name during recording.
    pub record_name_input: String,
    /// Current phase within the Services screen workflow.
    pub services_phase: ServicesPhase,
    /// Text input buffer for new service name entry.
    pub service_name_input: String,
    /// Cursor position within the combo profile picker during AssignCombo phase.
    pub services_assign_cursor: usize,
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
pub struct ServiceEntry {
    pub name: String,
    pub username: String,
    pub combo_hint: String,
    pub mock_secret: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ComboTestResult {
    Waiting,
    Match(String),
    NoMatch,
    InvalidInput,
    /// Sequence matched but inter-keypress timing fell outside the tolerance band.
    TimingMismatch,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VaultState {
    Locked,
    Unlocked {
        service: String,
        placeholder: String,
    },
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

impl Default for App {
    fn default() -> Self {
        Self {
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
            services: vec![
                ServiceEntry {
                    name: "GitHub".to_owned(),
                    username: "demo.dev".to_owned(),
                    combo_hint: "down right A".to_owned(),
                    mock_secret: "***mock-gh-token-abc123***".to_owned(),
                },
                ServiceEntry {
                    name: "Research Wiki".to_owned(),
                    username: "astro.local".to_owned(),
                    combo_hint: "left right B".to_owned(),
                    mock_secret: "***mock-wiki-pass-xyz789***".to_owned(),
                },
                ServiceEntry {
                    name: "Lab Notes".to_owned(),
                    username: "mock-user".to_owned(),
                    combo_hint: "up down X".to_owned(),
                    mock_secret: "***mock-lab-key-def456***".to_owned(),
                },
            ],
            combo_profiles: vec![
                ComboProfile {
                    name: "Quarter Turn".to_owned(),
                    sequence: "down right A".to_owned(),
                    status: "parsed".to_owned(),
                    timing_window_ms: 300,
                    gaps_ms: vec![],
                },
                ComboProfile {
                    name: "Dash Confirm".to_owned(),
                    sequence: "left right B".to_owned(),
                    status: "mock".to_owned(),
                    timing_window_ms: 400,
                    gaps_ms: vec![],
                },
                ComboProfile {
                    name: "Focus Reset".to_owned(),
                    sequence: "up down X".to_owned(),
                    status: "mock".to_owned(),
                    timing_window_ms: 500,
                    gaps_ms: vec![],
                },
            ],
            settings: vec![
                SettingEntry {
                    name: "Timing Tolerance",
                    value: "40%",
                },
                SettingEntry {
                    name: "Theme",
                    value: "terminal default",
                },
                SettingEntry {
                    name: "Secret Handling",
                    value: "disabled",
                },
            ],
            demo_combo: Combo::parse("down right A"),
            recorded_combo_tokens: Vec::new(),
            recorded_timestamps: Vec::new(),
            timing_tolerance_pct: 40,
            test_result: ComboTestResult::Waiting,
            vault_state: VaultState::Locked,
            record_phase: RecordPhase::NameEntry,
            record_name_input: String::new(),
            services_phase: ServicesPhase::List,
            service_name_input: String::new(),
            services_assign_cursor: 0,
        }
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
        self.lock_vault_on_exit();
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
        self.lock_vault_on_exit();
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
        self.lock_vault_on_exit();
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
        if count == 0 {
            return;
        }

        let selected = self.selected_index_mut();
        *selected = (*selected + 1) % count;
    }

    pub fn previous_item(&mut self) {
        let count = self.item_count();
        if count == 0 {
            return;
        }

        let selected = self.selected_index_mut();
        *selected = if *selected == 0 {
            count - 1
        } else {
            *selected - 1
        };
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
            // Numpad-style diagonal shortcuts (7/9/1/3 match numpad corners)
            '7' => "up-left",
            '9' => "up-right",
            '1' => "down-left",
            '3' => "down-right",
            other => {
                // Record any other printable char as-is (uppercase for letters)
                let s = if other.is_ascii_alphabetic() {
                    other.to_ascii_uppercase().to_string()
                } else {
                    other.to_string()
                };
                self.recorded_combo_tokens.push(s);
                self.recorded_timestamps.push(Instant::now());
                self.test_result = ComboTestResult::Waiting;
                self.vault_state = VaultState::Locked;
                return true;
            }
        };

        self.recorded_combo_tokens.push(token.to_owned());
        self.recorded_timestamps.push(Instant::now());
        self.test_result = ComboTestResult::Waiting;
        self.vault_state = VaultState::Locked;
        true
    }

    pub fn record_combo_token(&mut self, token: &str) {
        if !self.can_record_combo_tokens() {
            return;
        }
        self.recorded_combo_tokens.push(token.to_owned());
        self.recorded_timestamps.push(Instant::now());
        self.test_result = ComboTestResult::Waiting;
        self.vault_state = VaultState::Locked;
    }

    pub fn pop_recorded_combo_token(&mut self) {
        if self.can_record_combo_tokens() {
            self.recorded_combo_tokens.pop();
            self.recorded_timestamps.pop();
            self.test_result = ComboTestResult::Waiting;
            self.vault_state = VaultState::Locked;
        }
    }

    pub fn clear_recorded_combo(&mut self) {
        if self.can_record_combo_tokens() {
            self.recorded_combo_tokens.clear();
            self.recorded_timestamps.clear();
            self.test_result = ComboTestResult::Waiting;
            self.vault_state = VaultState::Locked;
        }
    }

    pub fn test_recorded_combo(&mut self) {
        let Some(recorded) = Combo::parse(&self.recorded_combo_input()) else {
            self.recorded_combo_tokens.clear();
            self.recorded_timestamps.clear();
            self.test_result = ComboTestResult::InvalidInput;
            return;
        };

        let test_gaps = self.recorded_gaps_ms();

        // Blind match: find the first profile whose sequence (and timing, if set) matches.
        let matched = self.combo_profiles.iter().find_map(|profile| {
            let expected = Combo::parse(&profile.sequence)?;
            if recorded != expected {
                return None;
            }
            let timing_ok = if profile.gaps_ms.is_empty() {
                true
            } else {
                gaps_pass_tolerance(&test_gaps, &profile.gaps_ms, self.timing_tolerance_pct)
            };
            if timing_ok {
                Some((profile.name.clone(), profile.sequence.clone()))
            } else {
                None
            }
        });

        // Detect sequence-only match (timing mismatch) before clearing tokens.
        let sequence_matched_any = matched.is_none()
            && self
                .combo_profiles
                .iter()
                .any(|p| Combo::parse(&p.sequence).map(|e| recorded == e).unwrap_or(false));

        self.recorded_combo_tokens.clear();
        self.recorded_timestamps.clear();

        if let Some((name, sequence)) = matched {
            self.vault_state = self.unlock_vault_for_sequence(&sequence);
            self.test_result = ComboTestResult::Match(name);
        } else if sequence_matched_any {
            self.test_result = ComboTestResult::TimingMismatch;
            self.vault_state = VaultState::Locked;
        } else {
            self.test_result = ComboTestResult::NoMatch;
            self.vault_state = VaultState::Locked;
        }
    }

    pub fn recorded_combo_input(&self) -> String {
        self.recorded_combo_tokens.join(" ")
    }

    /// Compute inter-keypress gaps in milliseconds from the recorded timestamps.
    /// Returns an empty vec if fewer than two timestamps are present.
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

    /// Enter the combo recording screen, resetting all transient recording state.
    pub fn start_record_combo(&mut self) {
        self.current_screen = Screen::RecordCombo;
        self.record_phase = RecordPhase::NameEntry;
        self.record_name_input.clear();
        self.recorded_combo_tokens.clear();
        self.recorded_timestamps.clear();
        self.test_result = ComboTestResult::Waiting;
        self.vault_state = VaultState::Locked;
    }

    /// Append a printable character to the name input (max 40 chars).
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

    /// Remove the last character from the name input.
    pub fn record_name_backspace(&mut self) {
        if self.current_screen == Screen::RecordCombo
            && self.record_phase == RecordPhase::NameEntry
        {
            self.record_name_input.pop();
        }
    }

    /// Advance from NameEntry to TokenCapture if the name is non-empty.
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

    /// Save the recorded combo as a new profile and return to the Combos screen.
    /// No-op if there are no tokens or the name is blank.
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
        self.combo_profiles.push(ComboProfile {
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

    /// Cancel recording and return to the Combos screen.
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
        if !self.is_services_add_name() {
            return;
        }
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
        if !self.is_services_add_name() {
            return;
        }
        let name = self.service_name_input.trim().to_owned();
        if name.is_empty() {
            return;
        }
        self.services.push(ServiceEntry {
            name,
            username: String::new(),
            combo_hint: String::new(),
            mock_secret: String::new(),
        });
        self.selected_detail_item = self.services.len() - 1;
        self.services_phase = ServicesPhase::List;
        self.service_name_input.clear();
    }

    pub fn start_assign_combo(&mut self) {
        if self.current_screen != Screen::Services
            || self.services_phase != ServicesPhase::List
            || self.combo_profiles.is_empty()
            || self.services.is_empty()
        {
            return;
        }
        self.services_phase = ServicesPhase::AssignCombo;
        self.services_assign_cursor = 0;
    }

    pub fn confirm_assign_combo(&mut self) {
        if !self.is_services_assign_combo() {
            return;
        }
        if self.combo_profiles.is_empty() || self.services.is_empty() {
            return;
        }
        let sequence = self.combo_profiles[self.services_assign_cursor].sequence.clone();
        self.services[self.selected_detail_item].combo_hint = sequence;
        self.services_phase = ServicesPhase::List;
    }

    pub fn cancel_services_action(&mut self) {
        if self.current_screen == Screen::Services && self.services_phase != ServicesPhase::List {
            self.services_phase = ServicesPhase::List;
            self.service_name_input.clear();
        }
    }

    fn cancel_record_combo_inner(&mut self, destination: Screen) {
        self.current_screen = destination;
        self.record_phase = RecordPhase::NameEntry;
        self.record_name_input.clear();
        self.recorded_combo_tokens.clear();
        self.recorded_timestamps.clear();
        self.test_result = ComboTestResult::Waiting;
        self.vault_state = VaultState::Locked;
    }

    fn lock_vault_on_exit(&mut self) {
        self.vault_state = VaultState::Locked;
        self.test_result = ComboTestResult::Waiting;
    }

    fn unlock_vault_for_sequence(&self, sequence: &str) -> VaultState {
        for service in &self.services {
            if service.combo_hint == sequence {
                return VaultState::Unlocked {
                    service: service.name.clone(),
                    placeholder: service.mock_secret.clone(),
                };
            }
        }
        VaultState::Locked
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
                _ => self.services.len(),
            },
            Screen::Combos => self.combo_profiles.len(),
            Screen::TestLab => 0,
            Screen::Settings => self.settings.len(),
            Screen::RecordCombo | Screen::Quit => 0,
        }
    }
}

/// Returns true if every recorded gap falls within `expected_gap ± tolerance_pct%`.
/// Always returns true when `expected` is empty (no timing constraint).
pub(crate) fn gaps_pass_tolerance(recorded: &[u64], expected: &[u64], tolerance_pct: u32) -> bool {
    if expected.is_empty() {
        return true;
    }
    if recorded.len() != expected.len() {
        return false;
    }
    let tol = tolerance_pct as f64 / 100.0;
    recorded.iter().zip(expected.iter()).all(|(&got, &exp)| {
        let lo = (exp as f64 * (1.0 - tol)) as u64;
        let hi = (exp as f64 * (1.0 + tol)) as u64;
        got >= lo && got <= hi
    })
}

#[cfg(test)]
mod tests {
    use super::{App, ComboProfile, ComboTestResult, RecordPhase, Screen, ServicesPhase, VaultState, gaps_pass_tolerance};

    // --- existing navigation / combo tests ---

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

    #[test]
    fn records_and_tests_matching_combo() {
        let mut app = App::default();
        app.current_screen = Screen::TestLab;

        assert!(app.record_combo_shortcut('d'));
        assert!(app.record_combo_shortcut('r'));
        assert!(app.record_combo_shortcut('a'));
        app.test_recorded_combo();

        assert_eq!(app.recorded_combo_input(), "");
        assert_eq!(app.test_result, ComboTestResult::Match("Quarter Turn".to_owned()));
    }

    #[test]
    fn reports_non_matching_combo() {
        let mut app = App::default();
        app.current_screen = Screen::TestLab;

        assert!(app.record_combo_shortcut('u'));
        assert!(app.record_combo_shortcut('x'));
        app.test_recorded_combo();

        assert_eq!(app.test_result, ComboTestResult::NoMatch);
    }

    #[test]
    fn correct_combo_unlocks_matching_service_vault() {
        let mut app = App::default();
        app.current_screen = Screen::TestLab;

        assert!(app.record_combo_shortcut('d'));
        assert!(app.record_combo_shortcut('r'));
        assert!(app.record_combo_shortcut('a'));
        app.test_recorded_combo();

        assert_eq!(
            app.vault_state,
            VaultState::Unlocked {
                service: "GitHub".to_owned(),
                placeholder: "***mock-gh-token-abc123***".to_owned(),
            }
        );
    }

    #[test]
    fn wrong_combo_leaves_vault_locked() {
        let mut app = App::default();
        app.current_screen = Screen::TestLab;

        assert!(app.record_combo_shortcut('u'));
        assert!(app.record_combo_shortcut('x'));
        app.test_recorded_combo();

        assert_eq!(app.vault_state, VaultState::Locked);
    }

    #[test]
    fn clear_resets_vault_to_locked() {
        let mut app = App::default();
        app.current_screen = Screen::TestLab;

        assert!(app.record_combo_shortcut('d'));
        assert!(app.record_combo_shortcut('r'));
        assert!(app.record_combo_shortcut('a'));
        app.test_recorded_combo();
        assert!(matches!(app.vault_state, VaultState::Unlocked { .. }));

        app.clear_recorded_combo();
        assert_eq!(app.vault_state, VaultState::Locked);
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

        // Any printable char is now accepted; letters are uppercased
        assert!(app.record_combo_shortcut('z'));
        assert_eq!(app.recorded_combo_input(), "Z");
        assert!(app.record_combo_shortcut('0'));
        assert_eq!(app.recorded_combo_input(), "Z 0");
    }

    #[test]
    fn second_service_combo_unlocks_correct_vault_entry() {
        let mut app = App::default();
        app.current_screen = Screen::TestLab;

        assert!(app.record_combo_shortcut('l'));
        assert!(app.record_combo_shortcut('r'));
        assert!(app.record_combo_shortcut('b'));
        app.test_recorded_combo();

        assert_eq!(
            app.vault_state,
            VaultState::Unlocked {
                service: "Research Wiki".to_owned(),
                placeholder: "***mock-wiki-pass-xyz789***".to_owned(),
            }
        );
    }

    #[test]
    fn vault_locks_when_navigating_home() {
        let mut app = App::default();
        app.current_screen = Screen::TestLab;
        app.record_combo_shortcut('d');
        app.record_combo_shortcut('r');
        app.record_combo_shortcut('a');
        app.test_recorded_combo();
        assert!(matches!(app.vault_state, VaultState::Unlocked { .. }));

        app.go_home();

        assert_eq!(app.vault_state, VaultState::Locked);
        assert_eq!(app.test_result, ComboTestResult::Waiting);
    }

    #[test]
    fn vault_locks_when_cycling_next_screen() {
        let mut app = App::default();
        app.current_screen = Screen::TestLab;
        app.record_combo_shortcut('d');
        app.record_combo_shortcut('r');
        app.record_combo_shortcut('a');
        app.test_recorded_combo();
        assert!(matches!(app.vault_state, VaultState::Unlocked { .. }));

        app.next_screen();

        assert_eq!(app.vault_state, VaultState::Locked);
        assert_eq!(app.test_result, ComboTestResult::Waiting);
    }

    #[test]
    fn vault_locks_when_cycling_previous_screen() {
        let mut app = App::default();
        app.current_screen = Screen::TestLab;
        app.record_combo_shortcut('d');
        app.record_combo_shortcut('r');
        app.record_combo_shortcut('a');
        app.test_recorded_combo();
        assert!(matches!(app.vault_state, VaultState::Unlocked { .. }));

        app.previous_screen();

        assert_eq!(app.vault_state, VaultState::Locked);
        assert_eq!(app.test_result, ComboTestResult::Waiting);
    }

    // --- timestamps / gap tracking ---

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

        // 3 tokens → 2 gaps
        assert_eq!(app.recorded_gaps_ms().len(), 2);
    }

    // --- timing with tolerance: profile that has recorded gaps ---

    fn make_app_with_timed_profile(gaps_ms: Vec<u64>) -> App {
        let mut app = App::default();
        app.current_screen = Screen::TestLab;
        // Replace the first profile with one that has gap constraints
        app.combo_profiles[0] = ComboProfile {
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

        assert_eq!(app.test_result, ComboTestResult::Match("Quarter Turn".to_owned()));
    }

    #[test]
    fn timing_mismatch_when_gaps_outside_tolerance() {
        let mut app = make_app_with_timed_profile(vec![200, 200]);
        // Inject timestamps far outside tolerance by manually setting them.
        // Simulated: record tokens, then replace timestamps with ones that give
        // ~500 ms gaps, which is 150% of 200 ms — well beyond 40%.
        use std::time::{Duration, Instant};
        let t0 = Instant::now();
        app.record_combo_token("down");
        app.record_combo_token("right");
        app.record_combo_token("A");
        // Overwrite timestamps: t0, t0+500ms, t0+1000ms → gaps [500, 500]
        app.recorded_timestamps = vec![
            t0,
            t0 + Duration::from_millis(500),
            t0 + Duration::from_millis(1000),
        ];

        app.test_recorded_combo();

        assert_eq!(app.test_result, ComboTestResult::TimingMismatch);
        assert_eq!(app.vault_state, VaultState::Locked);
    }

    #[test]
    fn timing_match_passes_within_tolerance() {
        let mut app = make_app_with_timed_profile(vec![200, 200]);
        use std::time::{Duration, Instant};
        let t0 = Instant::now();
        app.record_combo_token("down");
        app.record_combo_token("right");
        app.record_combo_token("A");
        // gaps [210, 220] — within 40% of 200 ms (tolerance band: [120, 280])
        app.recorded_timestamps = vec![
            t0,
            t0 + Duration::from_millis(210),
            t0 + Duration::from_millis(430),
        ];

        app.test_recorded_combo();

        assert_eq!(app.test_result, ComboTestResult::Match("Quarter Turn".to_owned()));
    }

    // --- gaps_pass_tolerance unit tests ---

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
        // band at 40%: [120, 280] for exp=200
        assert!(gaps_pass_tolerance(&[250], &[200], 40));
        assert!(gaps_pass_tolerance(&[130], &[200], 40));
    }

    #[test]
    fn tolerance_outside_band_fails() {
        // 300 is 50% above 200 → outside ±40%
        assert!(!gaps_pass_tolerance(&[300], &[200], 40));
        // 100 is 50% below 200 → outside ±40%
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

        // 3 tokens → 2 gaps
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
        let initial = App::default().services.len();
        let mut app = App::default();
        app.current_screen = Screen::Services;
        app.start_add_service();
        app.service_name_push_char('M');
        app.service_name_push_char('y');
        app.service_name_push_char('S');
        app.service_name_push_char('v');
        app.service_name_push_char('c');

        app.save_new_service();

        assert_eq!(app.services.len(), initial + 1);
        assert_eq!(app.services.last().unwrap().name, "MySvc");
        assert_eq!(app.services_phase, ServicesPhase::List);
        assert_eq!(app.selected_detail_item, initial);
    }

    #[test]
    fn save_new_service_noop_when_name_blank() {
        let initial = App::default().services.len();
        let mut app = App::default();
        app.current_screen = Screen::Services;
        app.start_add_service();

        app.save_new_service();

        assert_eq!(app.services.len(), initial);
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
    fn confirm_assign_combo_sets_combo_hint_on_service() {
        let mut app = App::default();
        app.current_screen = Screen::Services;
        app.selected_detail_item = 0;
        app.start_assign_combo();
        // cursor at 0 → "Quarter Turn" with sequence "down right A"
        app.services_assign_cursor = 0;

        app.confirm_assign_combo();

        assert_eq!(app.services[0].combo_hint, "down right A");
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

        for _ in 0..50 {
            app.service_name_push_char('a');
        }

        assert_eq!(app.service_name_input.len(), 40);
    }

    #[test]
    fn record_combo_name_max_length_enforced() {
        let mut app = App::default();
        app.start_record_combo();

        for _ in 0..50 {
            app.record_name_push_char('a');
        }

        assert_eq!(app.record_name_input.len(), 40);
    }

}