use crossterm::event::Event;
use ratatui::Frame;

pub enum ScreenAction {
    None,
    Quit,
    Push(Box<dyn Screen>),
    Pop,
    Replace(Box<dyn Screen>),
}

pub trait Screen {
    fn draw(&self, frame: &mut Frame);
    fn handle(&mut self, event: Event) -> anyhow::Result<ScreenAction>;
}
