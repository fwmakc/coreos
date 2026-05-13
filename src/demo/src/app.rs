//! Application state and business logic.

pub const CURSOR_COLOR: [f32; 4] = [0.0, 0.9, 1.0, 1.0];
pub const CURSOR_SIZE_MIN: f32 = 4.0;
pub const CURSOR_SIZE_MAX: f32 = 64.0;
pub const CURSOR_SIZE_DEFAULT: f32 = 16.0;

pub const CIRCLE_COLORS: [[f32; 4]; 5] = [
    [1.0, 0.2, 0.2, 1.0],
    [0.2, 1.0, 0.2, 1.0],
    [0.2, 0.2, 1.0, 1.0],
    [1.0, 1.0, 0.2, 1.0],
    [1.0, 0.2, 1.0, 1.0],
];

pub const FONT_SIZE: f32 = 32.0;
pub const CMD_FONT_SIZE: f32 = 24.0;
pub const TEXT_COLOR: [f32; 4] = [1.0, 1.0, 1.0, 1.0];
pub const CMD_PANEL_COLOR: [f32; 4] = [0.1, 0.12, 0.18, 0.95];
pub const CMD_PANEL_HEIGHT: f32 = 48.0;

pub const MAX_CIRCLES: usize = 10_000;
pub const MAX_HISTORY: usize = 100;
pub const HELP_TEXT: &str = "Commands: clear | exit | help | resize <N> | circles";

#[derive(Clone, Copy, Debug)]
pub struct Circle {
    pub x: f32,
    pub y: f32,
    pub radius: f32,
    pub color: [f32; 4],
}

/// Result of executing a command.
#[derive(Clone, Debug, PartialEq)]
pub enum CommandResult {
    Exit,
    ClearCircles,
    ResizeCursor(f32),
    ShowHelp,
    Unknown(String),
}

/// Which text buffer to modify.
#[derive(Clone, Copy, Debug)]
pub enum TextTarget {
    Typed,
    Command,
}

/// Pure application state. Knows nothing about wgpu or winit.
pub struct AppState {
    cursor_x: f64,
    cursor_y: f64,
    cursor_size: f32,
    circles: Vec<Circle>,
    next_color_idx: usize,
    pub command_bar_visible: bool,
    pub help_visible: bool,
    command_text: String,
    typed_text: String,
    history: Vec<String>,
    history_idx: Option<usize>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            cursor_x: 0.0,
            cursor_y: 0.0,
            cursor_size: CURSOR_SIZE_DEFAULT,
            circles: Vec::new(),
            next_color_idx: 0,
            command_bar_visible: false,
            help_visible: false,
            command_text: String::new(),
            typed_text: String::new(),
            history: Vec::new(),
            history_idx: None,
        }
    }
}

impl AppState {
    pub fn cursor_pos_x(&self) -> f64 {
        self.cursor_x
    }

    pub fn cursor_pos_y(&self) -> f64 {
        self.cursor_y
    }

    pub fn set_cursor_pos(&mut self, x: f64, y: f64) {
        self.cursor_x = x;
        self.cursor_y = y;
    }

    pub fn cursor_size(&self) -> f32 {
        self.cursor_size
    }

    pub fn circles(&self) -> &[Circle] {
        &self.circles
    }

    pub fn typed_text(&self) -> &str {
        &self.typed_text
    }

    pub fn command_text(&self) -> &str {
        &self.command_text
    }

    pub fn add_circle(&mut self) -> bool {
        if self.circles.len() >= MAX_CIRCLES {
            return false;
        }
        let color = CIRCLE_COLORS[self.next_color_idx];
        self.next_color_idx = (self.next_color_idx + 1) % CIRCLE_COLORS.len();
        self.circles.push(Circle {
            x: self.cursor_x as f32,
            y: self.cursor_y as f32,
            radius: self.cursor_size,
            color,
        });
        true
    }

    pub fn clear_circles(&mut self) {
        self.circles.clear();
        self.next_color_idx = 0;
    }

    pub fn resize_cursor(&mut self, delta: f32) {
        self.cursor_size = (self.cursor_size + delta).clamp(CURSOR_SIZE_MIN, CURSOR_SIZE_MAX);
    }

    pub fn toggle_command_bar(&mut self) {
        self.command_bar_visible = !self.command_bar_visible;
    }

    pub fn type_char(&mut self, ch: char, target: TextTarget) {
        match target {
            TextTarget::Typed => self.typed_text.push(ch),
            TextTarget::Command => self.command_text.push(ch),
        }
    }

    pub fn backspace(&mut self, target: TextTarget) {
        match target {
            TextTarget::Typed => {
                self.typed_text.pop();
            }
            TextTarget::Command => {
                self.command_text.pop();
            }
        }
    }

    pub fn execute_command(&mut self) -> Option<CommandResult> {
        if self.command_text.is_empty() {
            return None;
        }
        let input = std::mem::take(&mut self.command_text);
        let parts: Vec<&str> = input.split_whitespace().collect();
        let result = match parts.first().map(|w| w.to_lowercase()).as_deref() {
            Some("exit") | Some("quit") => {
                self.push_history(input);
                CommandResult::Exit
            }
            Some("clear") => {
                self.push_history(input);
                CommandResult::ClearCircles
            }
            Some("help") | Some("?") => {
                self.help_visible = !self.help_visible;
                CommandResult::ShowHelp
            }
            Some("resize") => {
                let size = parts.get(1).and_then(|v| v.parse::<f32>().ok());
                match size {
                    Some(n) => {
                        self.cursor_size = n.clamp(CURSOR_SIZE_MIN, CURSOR_SIZE_MAX);
                        self.push_history(input);
                        CommandResult::ResizeCursor(self.cursor_size)
                    }
                    None => CommandResult::Unknown(input),
                }
            }
            _ => CommandResult::Unknown(input),
        };
        Some(result)
    }

    fn push_history(&mut self, entry: String) {
        if self.history.len() >= MAX_HISTORY {
            self.history.remove(0);
        }
        self.history.push(entry);
    }

    pub fn history_up(&mut self) {
        if self.history.is_empty() {
            return;
        }
        let idx = self
            .history_idx
            .map_or(self.history.len() - 1, |i| i.saturating_sub(1));
        if idx < self.history.len() {
            self.history_idx = Some(idx);
            self.command_text = self.history[idx].clone();
        }
    }

    pub fn history_down(&mut self) {
        let Some(idx) = self.history_idx else { return };
        if idx + 1 < self.history.len() {
            self.history_idx = Some(idx + 1);
            self.command_text = self.history[idx + 1].clone();
        } else {
            self.history_idx = None;
            self.command_text.clear();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cursor_pos_default() {
        let s = AppState::default();
        assert_eq!(s.cursor_pos_x(), 0.0);
        assert_eq!(s.cursor_pos_y(), 0.0);
    }

    #[test]
    fn set_cursor_pos() {
        let mut s = AppState::default();
        s.set_cursor_pos(100.0, 200.0);
        assert_eq!(s.cursor_pos_x(), 100.0);
        assert_eq!(s.cursor_pos_y(), 200.0);
    }

    #[test]
    fn resize_cursor_clamps() {
        let mut s = AppState::default();
        assert_eq!(s.cursor_size(), CURSOR_SIZE_DEFAULT);

        s.resize_cursor(-100.0);
        assert_eq!(s.cursor_size(), CURSOR_SIZE_MIN);

        s.resize_cursor(1000.0);
        assert_eq!(s.cursor_size(), CURSOR_SIZE_MAX);
    }

    #[test]
    fn add_circle_cycles_colors() {
        let mut s = AppState::default();
        s.set_cursor_pos(50.0, 60.0);
        s.resize_cursor(10.0);

        s.add_circle();
        assert_eq!(s.circles().len(), 1);
        assert_eq!(s.circles()[0].color, CIRCLE_COLORS[0]);

        s.add_circle();
        assert_eq!(s.circles()[1].color, CIRCLE_COLORS[1]);
    }

    #[test]
    fn clear_circles_resets_color_index() {
        let mut s = AppState::default();
        s.add_circle();
        s.add_circle();
        s.add_circle();
        assert_eq!(s.circles().len(), 3);
        s.clear_circles();
        assert!(s.circles().is_empty());
        s.add_circle();
        assert_eq!(s.circles()[0].color, CIRCLE_COLORS[0]);
    }

    #[test]
    fn execute_command_returns_and_clears() {
        let mut s = AppState::default();
        assert!(s.execute_command().is_none());

        s.type_char('h', TextTarget::Command);
        s.type_char('i', TextTarget::Command);
        let result = s.execute_command().unwrap();
        assert_eq!(result, CommandResult::Unknown("hi".to_owned()));
        assert!(s.command_text().is_empty());
    }

    #[test]
    fn execute_command_exit() {
        let mut s = AppState::default();
        s.type_char('e', TextTarget::Command);
        s.type_char('x', TextTarget::Command);
        s.type_char('i', TextTarget::Command);
        s.type_char('t', TextTarget::Command);
        assert_eq!(s.execute_command().unwrap(), CommandResult::Exit);
    }

    #[test]
    fn execute_command_clear() {
        let mut s = AppState::default();
        for c in "clear".chars() {
            s.type_char(c, TextTarget::Command);
        }
        assert_eq!(s.execute_command().unwrap(), CommandResult::ClearCircles);
    }

    #[test]
    fn execute_command_resize() {
        let mut s = AppState::default();
        for c in "resize 32".chars() {
            s.type_char(c, TextTarget::Command);
        }
        assert_eq!(
            s.execute_command().unwrap(),
            CommandResult::ResizeCursor(32.0)
        );
        assert_eq!(s.cursor_size(), 32.0);
    }

    #[test]
    fn execute_command_resize_clamps() {
        let mut s = AppState::default();
        for c in "resize 999".chars() {
            s.type_char(c, TextTarget::Command);
        }
        assert_eq!(
            s.execute_command().unwrap(),
            CommandResult::ResizeCursor(CURSOR_SIZE_MAX)
        );
    }

    #[test]
    fn execute_command_help() {
        let mut s = AppState::default();
        for c in "help".chars() {
            s.type_char(c, TextTarget::Command);
        }
        assert_eq!(s.execute_command().unwrap(), CommandResult::ShowHelp);
    }

    #[test]
    fn add_circle_enforces_limit() {
        let mut s = AppState::default();
        s.set_cursor_pos(10.0, 10.0);
        for _ in 0..MAX_CIRCLES {
            assert!(s.add_circle());
        }
        assert!(!s.add_circle());
    }

    #[test]
    fn backspace_removes_last_char() {
        let mut s = AppState::default();
        s.type_char('a', TextTarget::Typed);
        s.type_char('b', TextTarget::Typed);
        s.backspace(TextTarget::Typed);
        assert_eq!(s.typed_text(), "a");
    }

    #[test]
    fn toggle_command_bar() {
        let mut s = AppState::default();
        assert!(!s.command_bar_visible);
        s.toggle_command_bar();
        assert!(s.command_bar_visible);
        s.toggle_command_bar();
        assert!(!s.command_bar_visible);
    }
}
