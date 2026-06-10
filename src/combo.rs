#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Combo {
    steps: Vec<ComboStep>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ComboStep {
    Direction(Direction),
    Button(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Up,
    Down,
    Left,
    Right,
    UpRight,
    DownRight,
    DownLeft,
    UpLeft,
}

/// Timing constraint for combo input — how long the full sequence may take.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TimingWindow {
    pub window_ms: u32,
}

impl TimingWindow {
    pub fn new(window_ms: u32) -> Self {
        Self { window_ms }
    }
}

/// A combo paired with an input timing constraint.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimedCombo {
    pub combo: Combo,
    pub timing: TimingWindow,
}

impl TimedCombo {
    pub fn new(combo: Combo, window_ms: u32) -> Self {
        Self {
            combo,
            timing: TimingWindow::new(window_ms),
        }
    }
}

impl Combo {
    pub fn parse(input: &str) -> Option<Self> {
        let steps: Option<Vec<ComboStep>> = input.split_whitespace().map(parse_step).collect();

        steps
            .filter(|steps| !steps.is_empty())
            .map(|steps| Self { steps })
    }

    pub fn steps(&self) -> &[ComboStep] {
        &self.steps
    }

    pub fn len(&self) -> usize {
        self.steps.len()
    }

    pub fn is_empty(&self) -> bool {
        self.steps.is_empty()
    }
}

fn parse_step(token: &str) -> Option<ComboStep> {
    let lower = token.to_ascii_lowercase();
    match lower.as_str() {
        "up" | "u" => Some(ComboStep::Direction(Direction::Up)),
        "down" | "d" => Some(ComboStep::Direction(Direction::Down)),
        "left" | "l" => Some(ComboStep::Direction(Direction::Left)),
        "right" | "r" => Some(ComboStep::Direction(Direction::Right)),
        "up-right" | "upright" | "ur" => Some(ComboStep::Direction(Direction::UpRight)),
        "down-right" | "downright" | "dr" => Some(ComboStep::Direction(Direction::DownRight)),
        "down-left" | "downleft" | "dl" => Some(ComboStep::Direction(Direction::DownLeft)),
        "up-left" | "upleft" | "ul" => Some(ComboStep::Direction(Direction::UpLeft)),
        "l1" | "l2" | "r1" | "r2" | "lt" | "rt" | "lb" | "rb" | "start" | "select" => {
            Some(ComboStep::Button(token.to_ascii_uppercase()))
        }
        single if single.len() == 1 && single.chars().next()?.is_ascii_alphabetic() => {
            Some(ComboStep::Button(single.to_ascii_uppercase()))
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::{Combo, ComboStep, Direction, TimedCombo, TimingWindow};

    // --- valid combos ---

    #[test]
    fn parses_direction_and_button_sequence() {
        let combo = Combo::parse("down right A").expect("valid combo");

        assert_eq!(
            combo.steps(),
            &[
                ComboStep::Direction(Direction::Down),
                ComboStep::Direction(Direction::Right),
                ComboStep::Button("A".into()),
            ]
        );
    }

    #[test]
    fn parses_diagonal_tokens() {
        let combo = Combo::parse("down-right up-left dr ul").expect("valid combo");

        assert_eq!(
            combo.steps(),
            &[
                ComboStep::Direction(Direction::DownRight),
                ComboStep::Direction(Direction::UpLeft),
                ComboStep::Direction(Direction::DownRight),
                ComboStep::Direction(Direction::UpLeft),
            ]
        );
    }

    #[test]
    fn parses_all_diagonal_aliases() {
        let cases = [
            ("up-right", Direction::UpRight),
            ("upright", Direction::UpRight),
            ("ur", Direction::UpRight),
            ("down-right", Direction::DownRight),
            ("downright", Direction::DownRight),
            ("dr", Direction::DownRight),
            ("down-left", Direction::DownLeft),
            ("downleft", Direction::DownLeft),
            ("dl", Direction::DownLeft),
            ("up-left", Direction::UpLeft),
            ("upleft", Direction::UpLeft),
            ("ul", Direction::UpLeft),
        ];

        for (token, expected) in cases {
            let combo = Combo::parse(token).expect(token);
            assert_eq!(
                combo.steps(),
                &[ComboStep::Direction(expected)],
                "failed for token: {token}"
            );
        }
    }

    #[test]
    fn parses_named_buttons() {
        let combo = Combo::parse("down dr L1 R2 start").expect("valid combo");

        assert_eq!(
            combo.steps(),
            &[
                ComboStep::Direction(Direction::Down),
                ComboStep::Direction(Direction::DownRight),
                ComboStep::Button("L1".into()),
                ComboStep::Button("R2".into()),
                ComboStep::Button("START".into()),
            ]
        );
    }

    #[test]
    fn parses_short_aliases() {
        let combo = Combo::parse("u d l r").expect("valid combo");

        assert_eq!(
            combo.steps(),
            &[
                ComboStep::Direction(Direction::Up),
                ComboStep::Direction(Direction::Down),
                ComboStep::Direction(Direction::Left),
                ComboStep::Direction(Direction::Right),
            ]
        );
    }

    #[test]
    fn parses_single_button() {
        let combo = Combo::parse("B").expect("valid combo");
        assert_eq!(combo.steps(), &[ComboStep::Button("B".into())]);
    }

    #[test]
    fn combo_len_matches_step_count() {
        let combo = Combo::parse("down right A").expect("valid combo");
        assert_eq!(combo.len(), 3);
        assert!(!combo.is_empty());
    }

    // --- invalid combos ---

    #[test]
    fn rejects_unknown_tokens() {
        assert!(Combo::parse("down spin A").is_none());
    }

    #[test]
    fn rejects_empty_input() {
        assert!(Combo::parse("   ").is_none());
    }

    #[test]
    fn rejects_numeric_only_token() {
        assert!(Combo::parse("1 2 3").is_none());
    }

    #[test]
    fn rejects_unrecognized_named_button() {
        // L3 is not in the named-button list
        assert!(Combo::parse("down L3").is_none());
    }

    // --- timing window ---

    #[test]
    fn timing_window_stores_ms() {
        let tw = TimingWindow::new(300);
        assert_eq!(tw.window_ms, 300);
    }

    #[test]
    fn timed_combo_wraps_combo_and_window() {
        let combo = Combo::parse("down dr A").expect("valid combo");
        let timed = TimedCombo::new(combo.clone(), 500);

        assert_eq!(timed.combo, combo);
        assert_eq!(timed.timing.window_ms, 500);
    }

    // --- partial / ambiguous ---

    #[test]
    fn single_step_combo_is_valid() {
        assert!(Combo::parse("A").is_some());
        assert!(Combo::parse("down").is_some());
    }

    #[test]
    fn equality_is_order_sensitive() {
        let a = Combo::parse("down right A").expect("valid");
        let b = Combo::parse("right down A").expect("valid");
        assert_ne!(a, b);
    }

    #[test]
    fn button_comparison_is_case_insensitive_after_parse() {
        let lower = Combo::parse("a").expect("valid");
        let upper = Combo::parse("A").expect("valid");
        assert_eq!(lower, upper);
    }
}
