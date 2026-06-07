#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Combo {
    steps: Vec<ComboStep>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ComboStep {
    Direction(Direction),
    Button(char),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Up,
    Down,
    Left,
    Right,
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
}

fn parse_step(token: &str) -> Option<ComboStep> {
    match token.to_ascii_lowercase().as_str() {
        "up" | "u" => Some(ComboStep::Direction(Direction::Up)),
        "down" | "d" => Some(ComboStep::Direction(Direction::Down)),
        "left" | "l" => Some(ComboStep::Direction(Direction::Left)),
        "right" | "r" => Some(ComboStep::Direction(Direction::Right)),
        button if button.len() == 1 => {
            let value = button.chars().next()?;
            Some(ComboStep::Button(value.to_ascii_uppercase()))
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::{Combo, ComboStep, Direction};

    #[test]
    fn parses_direction_and_button_sequence() {
        let combo = Combo::parse("down right A").expect("valid combo");

        assert_eq!(
            combo.steps(),
            &[
                ComboStep::Direction(Direction::Down),
                ComboStep::Direction(Direction::Right),
                ComboStep::Button('A')
            ]
        );
    }

    #[test]
    fn rejects_unknown_tokens() {
        assert!(Combo::parse("down spin A").is_none());
    }

    #[test]
    fn rejects_empty_input() {
        assert!(Combo::parse("   ").is_none());
    }
}
