use std::{
    io::Stdout,
    time::{Duration, Instant},
};

use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{self, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{Terminal, backend::CrosstermBackend};

use crate::{Screen, ScreenAction};

pub struct App {
    screens: Vec<Box<dyn Screen>>,
    terminal: Terminal<CrosstermBackend<Stdout>>,
    running: bool,
    tick_rate: Duration,
}

impl App {
    pub fn new(initial: impl Screen + 'static) -> anyhow::Result<Self> {
        terminal::enable_raw_mode()?;

        let mut stdout = std::io::stdout();

        crossterm::execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;

        let backend = CrosstermBackend::new(stdout);
        let terminal = Terminal::new(backend)?;

        Ok(Self {
            screens: vec![Box::new(initial)],
            terminal,
            running: true,
            tick_rate: Duration::from_millis(250),
        })
    }

    /// How often `Screen::update` is called (and the UI redrawn) when no input arrives.
    pub fn with_tick_rate(mut self, tick_rate: Duration) -> Self {
        self.tick_rate = tick_rate;
        self
    }

    pub fn run(&mut self) -> anyhow::Result<()> {
        let mut last_tick = Instant::now();

        while self.running {
            self.terminal.draw(|frame| {
                if let Some(screen) = self.screens.last_mut() {
                    screen.draw(frame);
                }
            })?;

            // Wait for input until the next tick is due; returns immediately if an event arrives.
            let timeout = self.tick_rate.saturating_sub(last_tick.elapsed());

            if event::poll(timeout)? {
                let event = event::read()?;

                if let Some(screen) = self.screens.last_mut() {
                    let action = screen.handle(event)?;
                    self.handle(action);
                }
            }

            if last_tick.elapsed() >= self.tick_rate {
                if let Some(screen) = self.screens.last_mut() {
                    let action = screen.update()?;
                    self.handle(action);
                }

                last_tick = Instant::now();
            }
        }

        Ok(())
    }

    fn handle(&mut self, action: ScreenAction) {
        match action {
            ScreenAction::None => {}
            ScreenAction::Quit => self.running = false,
            ScreenAction::Push(screen) => self.screens.push(screen),
            ScreenAction::Pop => {
                self.screens.pop();

                if self.screens.is_empty() {
                    self.running = false;
                }
            }
            ScreenAction::Replace(screen) => {
                self.screens.pop();
                self.screens.push(screen);
            }
        }
    }
}

impl Drop for App {
    fn drop(&mut self) {
        let _ = terminal::disable_raw_mode();

        let _ = execute!(
            self.terminal.backend_mut(),
            LeaveAlternateScreen,
            DisableMouseCapture
        );

        let _ = self.terminal.show_cursor();
    }
}
