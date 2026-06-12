use std::time::Instant;

use crate::combo::{Combo, MatchState, TimedCombo};

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
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    Home,
    Services,
    Combos,
    TestLab,
    Settings,
    Quit,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServiceEntry {
    pub name: &'static str,
    pub username: &'static str,
    pub combo_hint: &'static str,
    pub mock_secret: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComboProfile {
    pub name: &'static str,
    pub sequence: &'static str,
    pub status: &'static str,
    pub timing_window_ms: u32,
    /// Recorded inter-keypress gaps (ms) from the original recording session.
    /// Empty means no timing constraint is enforced at test time.
    pub gaps_ms: Vec<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ComboTestResult {
    Waiting,
    Match(&'static str),
    NoMatch,
    InvalidInput,
    /// Sequence matched but inter-keypress timing fell outside the tolerance band.
    TimingMismatch,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VaultState {
    Locked,
    Unlocked {
        service: &'static str,
        placeholder: &'static str,
    },
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
                    name: "GitHub",
                    username: "demo.dev",
                    combo_hint: "down right A",
                    mock_secret: "***mock-gh-token-abc123***",
                },
                ServiceEntry {
                    name: "Research Wiki",
                    username: "astro.local",
                    combo_hint: "left right B",
                    mock_secret: "***mock-wiki-pass-xyz789***",
                },
                ServiceEntry {
                    name: "Lab Notes",
                    username: "mock-user",
                    combo_hint: "up down X",
                    mock_secret: "***mock-lab-key-def456***",
                },
            ],
            combo_profiles: vec![
                ComboProfile {
                    name: "Quarter Turn",
                    sequence: "down right A",
                    status: "parsed",
                    timing_window_ms: 300,
                    gaps_ms: vec![],
                },
                ComboProfile {
                    name: "Dash Confirm",
                    sequence: "left right B",
                    status: "mock",
                    timing_window_ms: 400,
                    gaps_ms: vec![],
                },
                ComboProfile {
                    name: "Focus Reset",
                    sequence: "up down X",
                    status: "mock",
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
        self.lock_vault_on_exit();
        self.current_screen = Screen::Home;
        self.selected_detail_item = 0;
    }

    pub fn next_screen(&mut self) {
        self.lock_vault_on_exit();
        self.current_screen = match self.current_screen {
            Screen::Home => self.home_items[self.selected_home_item],
            Screen::Services => Screen::Combos,
            Screen::Combos => Screen::TestLab,
            Screen::TestLab => Screen::Settings,
            Screen::Settings => Screen::Services,
            Screen::Quit => Screen::Home,
        };
        self.selected_detail_item = 0;
    }

    pub fn previous_screen(&mut self) {
        self.lock_vault_on_exit();
        self.current_screen = match self.current_screen {
            Screen::Home => self.home_items[self.selected_home_item],
            Screen::Services => Screen::Settings,
            Screen::Combos => Screen::Services,
            Screen::TestLab => Screen::Combos,
            Screen::Settings => Screen::TestLab,
            Screen::Quit => Screen::Home,
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
        if self.current_screen != Screen::TestLab {
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
            _ => return false,
        };

        self.recorded_combo_tokens.push(token.to_owned());
        self.recorded_timestamps.push(Instant::now());
        self.test_result = ComboTestResult::Waiting;
        self.vault_state = VaultState::Locked;
        true
    }

    pub fn record_combo_token(&mut self, token: &str) {
        if self.current_screen != Screen::TestLab {
            return;
        }
        self.recorded_combo_tokens.push(token.to_owned());
        self.recorded_timestamps.push(Instant::now());
        self.test_result = ComboTestResult::Waiting;
        self.vault_state = VaultState::Locked;
    }

    pub fn pop_recorded_combo_token(&mut self) {
        if self.current_screen == Screen::TestLab {
            self.recorded_combo_tokens.pop();
            self.recorded_timestamps.pop();
            self.test_result = ComboTestResult::Waiting;
            self.vault_state = VaultState::Locked;
        }
    }

    pub fn clear_recorded_combo(&mut self) {
        if self.current_screen == Screen::TestLab {
            self.recorded_combo_tokens.clear();
            self.recorded_timestamps.clear();
            self.test_result = ComboTestResult::Waiting;
            self.vault_state = VaultState::Locked;
        }
    }

    pub fn load_selected_test_combo(&mut self) {
        let Some(profile) = self.selected_combo_profile() else {
            return;
        };

        self.recorded_combo_tokens = profile.sequence.split_whitespace().map(|s| s.to_owned()).collect();
        self.recorded_timestamps.clear();
        self.test_result = ComboTestResult::Waiting;
        self.vault_state = VaultState::Locked;
    }

    pub fn test_recorded_combo(&mut self) {
        let (profile_name, profile_sequence, profile_gaps) = match self.selected_combo_profile() {
            Some(p) => (p.name, p.sequence, p.gaps_ms.clone()),
            None => {
                self.test_result = ComboTestResult::InvalidInput;
                return;
            }
        };

        let Some(recorded) = Combo::parse(&self.recorded_combo_input()) else {
            self.test_result = ComboTestResult::InvalidInput;
            return;
        };

        let Some(expected) = Combo::parse(profile_sequence) else {
            self.test_result = ComboTestResult::InvalidInput;
            return;
        };

        if recorded == expected {
            let timing_ok = if profile_gaps.is_empty() {
                true
            } else {
                let test_gaps = self.recorded_gaps_ms();
                gaps_pass_tolerance(&test_gaps, &profile_gaps, self.timing_tolerance_pct)
            };

            if timing_ok {
                self.test_result = ComboTestResult::Match(profile_name);
                self.vault_state = self.unlock_vault_for_sequence(profile_sequence);
            } else {
                self.test_result = ComboTestResult::TimingMismatch;
                self.vault_state = VaultState::Locked;
            }
        } else {
            self.test_result = ComboTestResult::NoMatch;
            self.vault_state = VaultState::Locked;
        }

        self.recorded_combo_tokens.clear();
        self.recorded_timestamps.clear();
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

    pub fn selected_combo_profile(&self) -> Option<&ComboProfile> {
        self.combo_profiles.get(self.selected_detail_item)
    }

    pub fn selected_timed_combo(&self) -> Option<TimedCombo> {
        let profile = self.selected_combo_profile()?;
        let combo = Combo::parse(profile.sequence)?;
        Some(TimedCombo::new(combo, profile.timing_window_ms))
    }

    pub fn prefix_match_state(&self) -> Option<MatchState> {
        if self.recorded_combo_tokens.is_empty() {
            return None;
        }
        let profile = self.selected_combo_profile()?;
        let target = Combo::parse(profile.sequence)?;
        let partial = Combo::parse(&self.recorded_combo_input())?;
        Some(target.match_prefix(&partial))
    }

    fn lock_vault_on_exit(&mut self) {
        self.vault_state = VaultState::Locked;
        self.test_result = ComboTestResult::Waiting;
    }

    fn unlock_vault_for_sequence(&self, sequence: &str) -> VaultState {
        for service in &self.services {
            if service.combo_hint == sequence {
                return VaultState::Unlocked {
                    service: service.name,
                    placeholder: service.mock_secret,
                };
            }
        }
        VaultState::Locked
    }

    fn selected_index_mut(&mut self) -> &mut usize {
        if self.current_screen == Screen::Home {
            &mut self.selected_home_item
        } else {
            &mut self.selected_detail_item
        }
    }

    fn item_count(&self) -> usize {
        match self.current_screen {
            Screen::Home => self.home_items.len(),
            Screen::Services => self.services.len(),
            Screen::Combos => self.combo_profiles.len(),
            Screen::TestLab => self.combo_profiles.len(),
            Screen::Settings => self.settings.len(),
            Screen::Quit => 0,
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
    use super::{App, ComboProfile, ComboTestResult, Screen, VaultState, gaps_pass_tolerance};

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
        assert_eq!(app.test_result, ComboTestResult::Match("Quarter Turn"));
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
    fn can_load_selected_predefined_combo() {
        let mut app = App::default();
        app.current_screen = Screen::TestLab;
        app.selected_detail_item = 1;

        app.load_selected_test_combo();

        assert_eq!(app.recorded_combo_input(), "left right B");
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
                service: "GitHub",
                placeholder: "***mock-gh-token-abc123***",
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
    fn selected_timed_combo_returns_profile_combo_and_timing() {
        let app = App::default();

        let tc = app.selected_timed_combo().expect("first profile present");
        assert_eq!(tc.timing.window_ms, 300);
        assert_eq!(tc.combo.len(), 3);
    }

    #[test]
    fn selected_timed_combo_updates_with_selection() {
        let mut app = App::default();
        app.current_screen = Screen::TestLab;
        app.selected_detail_item = 1;

        let tc = app.selected_timed_combo().expect("second profile present");
        assert_eq!(tc.timing.window_ms, 400);
        assert_eq!(tc.combo.len(), 3);
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
    fn unknown_shortcut_key_is_rejected() {
        let mut app = App::default();
        app.current_screen = Screen::TestLab;

        assert!(!app.record_combo_shortcut('z'));
        assert_eq!(app.recorded_combo_input(), "");
    }

    #[test]
    fn second_service_combo_unlocks_correct_vault_entry() {
        let mut app = App::default();
        app.current_screen = Screen::TestLab;
        app.selected_detail_item = 1;

        assert!(app.record_combo_shortcut('l'));
        assert!(app.record_combo_shortcut('r'));
        assert!(app.record_combo_shortcut('b'));
        app.test_recorded_combo();

        assert_eq!(
            app.vault_state,
            VaultState::Unlocked {
                service: "Research Wiki",
                placeholder: "***mock-wiki-pass-xyz789***",
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
    fn load_selected_clears_timestamps() {
        let mut app = App::default();
        app.current_screen = Screen::TestLab;

        app.record_combo_shortcut('d');
        app.load_selected_test_combo();

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
            name: "Quarter Turn",
            sequence: "down right A",
            status: "parsed",
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

        assert_eq!(app.test_result, ComboTestResult::Match("Quarter Turn"));
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

        assert_eq!(app.test_result, ComboTestResult::Match("Quarter Turn"));
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
}
