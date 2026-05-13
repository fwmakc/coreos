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

#[derive(Clone, Copy, Debug)]
pub struct Circle {
    pub x: f32,
    pub y: f32,
    pub radius: f32,
    pub color: [f32; 4],
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
    command_text: String,
    typed_text: String,
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
            command_text: String::new(),
            typed_text: String::new(),
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

    pub fn add_circle(&mut self) {
        let color = CIRCLE_COLORS[self.next_color_idx];
        self.next_color_idx = (self.next_color_idx + 1) % CIRCLE_COLORS.len();
        self.circles.push(Circle {
            x: self.cursor_x as f32,
            y: self.cursor_y as f32,
            radius: self.cursor_size,
            color,
        });
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

    pub fn execute_command(&mut self) -> Option<String> {
        if self.command_text.is_empty() {
            return None;
        }
        Some(std::mem::take(&mut self.command_text))
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
        let cmd = s.execute_command().unwrap();
        assert_eq!(cmd, "hi");
        assert!(s.command_text().is_empty());
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
