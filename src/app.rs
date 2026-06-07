use crate::combo::Combo;

#[derive(Debug, Clone)]
pub struct App {
    pub should_quit: bool,
    pub selected_item: usize,
    pub menu_items: Vec<MenuItem>,
    pub demo_combo: Option<Combo>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuItem {
    Services,
    Combos,
    Quit,
}

impl MenuItem {
    pub fn label(self) -> &'static str {
        match self {
            MenuItem::Services => "Services",
            MenuItem::Combos => "Combos",
            MenuItem::Quit => "Quit",
        }
    }
}

impl Default for App {
    fn default() -> Self {
        Self {
            should_quit: false,
            selected_item: 0,
            menu_items: vec![MenuItem::Services, MenuItem::Combos, MenuItem::Quit],
            demo_combo: Combo::parse("down right A"),
        }
    }
}

impl App {
    pub fn quit(&mut self) {
        self.should_quit = true;
    }

    pub fn next_item(&mut self) {
        self.selected_item = (self.selected_item + 1) % self.menu_items.len();
    }

    pub fn previous_item(&mut self) {
        self.selected_item = if self.selected_item == 0 {
            self.menu_items.len() - 1
        } else {
            self.selected_item - 1
        };
    }
}
