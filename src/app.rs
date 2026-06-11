use crate::combo::{Combo, TimedCombo};

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
    pub recorded_combo_tokens: Vec<&'static str>,
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
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ComboTestResult {
    Waiting,
    Match(&'static str),
    NoMatch,
    InvalidInput,
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
                },
                ComboProfile {
                    name: "Dash Confirm",
                    sequence: "left right B",
                    status: "mock",
                    timing_window_ms: 400,
                },
                ComboProfile {
                    name: "Focus Reset",
                    sequence: "up down X",
                    status: "mock",
                    timing_window_ms: 500,
                },
            ],
            settings: vec![
                SettingEntry {
                    name: "Timing Window",
                    value: "300 ms mock",
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
        self.current_screen = Screen::Home;
        self.selected_detail_item = 0;
    }

    pub fn next_screen(&mut self) {
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
        self.current_screen = match self.current_screen {
            Screen::Home => self.home_items[self.selected_home_item],
            Screen::Services => Screen::Settings,
            Screen::Combos => Screen::Services,
            Screen::TestLab => Screen::Combos,
            Screen::Settings => Screen::Combos,
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
            _ => return false,
        };

        self.recorded_combo_tokens.push(token);
        self.test_result = ComboTestResult::Waiting;
        self.vault_state = VaultState::Locked;
        true
    }

    pub fn pop_recorded_combo_token(&mut self) {
        if self.current_screen == Screen::TestLab {
            self.recorded_combo_tokens.pop();
            self.test_result = ComboTestResult::Waiting;
            self.vault_state = VaultState::Locked;
        }
    }

    pub fn clear_recorded_combo(&mut self) {
        if self.current_screen == Screen::TestLab {
            self.recorded_combo_tokens.clear();
            self.test_result = ComboTestResult::Waiting;
            self.vault_state = VaultState::Locked;
        }
    }

    pub fn load_selected_test_combo(&mut self) {
        let Some(profile) = self.selected_combo_profile() else {
            return;
        };

        self.recorded_combo_tokens = profile.sequence.split_whitespace().collect();
        self.test_result = ComboTestResult::Waiting;
        self.vault_state = VaultState::Locked;
    }

    pub fn test_recorded_combo(&mut self) {
        let (profile_name, profile_sequence) = match self.selected_combo_profile() {
            Some(p) => (p.name, p.sequence),
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
            self.test_result = ComboTestResult::Match(profile_name);
            self.vault_state = self.unlock_vault_for_sequence(profile_sequence);
        } else {
            self.test_result = ComboTestResult::NoMatch;
            self.vault_state = VaultState::Locked;
        }
    }

    pub fn recorded_combo_input(&self) -> String {
        self.recorded_combo_tokens.join(" ")
    }

    pub fn selected_combo_profile(&self) -> Option<&ComboProfile> {
        self.combo_profiles.get(self.selected_detail_item)
    }

    pub fn selected_timed_combo(&self) -> Option<TimedCombo> {
        let profile = self.selected_combo_profile()?;
        let combo = Combo::parse(profile.sequence)?;
        Some(TimedCombo::new(combo, profile.timing_window_ms))
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

#[cfg(test)]
mod tests {
    use super::{App, ComboTestResult, Screen, VaultState};

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

        assert_eq!(app.recorded_combo_input(), "down right A");
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
}
