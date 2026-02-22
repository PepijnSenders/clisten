// Terminal backend: raw-mode setup, event polling, and tick generation.
// Wraps crossterm + ratatui so the rest of the app just sees key/resize/tick events.

use crossterm::{
    event::{self, Event as CrosstermEvent, KeyEvent, KeyEventKind},
    execute,
    terminal::{self, EnterAlternateScreen, LeaveAlternateScreen},
};
use futures_util::StreamExt;
use ratatui::{backend::CrosstermBackend, Terminal};
use std::time::Duration;
use tokio::sync::mpsc;

type CrosstermTerminal = Terminal<CrosstermBackend<std::io::Stderr>>;

pub struct Tui {
    terminal: CrosstermTerminal,
    pub event_rx: mpsc::UnboundedReceiver<TuiEvent>,
    event_tx: mpsc::UnboundedSender<TuiEvent>,
    frame_rate: f64,
}

#[derive(Debug)]
pub enum TuiEvent {
    Key(KeyEvent),
    Resize,
    Tick,
}

impl Tui {
    pub fn new(frame_rate: f64) -> anyhow::Result<Self> {
        let backend = CrosstermBackend::new(std::io::stderr());
        let terminal = Terminal::new(backend)?;
        let (event_tx, event_rx) = mpsc::unbounded_channel();
        Ok(Self {
            terminal,
            event_rx,
            event_tx,
            frame_rate,
        })
    }

    pub fn enter(&mut self) -> anyhow::Result<()> {
        terminal::enable_raw_mode()?;
        execute!(std::io::stderr(), EnterAlternateScreen)?;
        self.terminal.hide_cursor()?;
        self.terminal.clear()?;
        self.start_event_polling();
        Ok(())
    }

    pub fn exit(&mut self) -> anyhow::Result<()> {
        terminal::disable_raw_mode()?;
        execute!(std::io::stderr(), LeaveAlternateScreen)?;
        self.terminal.show_cursor()?;
        Ok(())
    }

    fn start_event_polling(&self) {
        let tx = self.event_tx.clone();
        let tick_rate = Duration::from_secs_f64(1.0 / self.frame_rate);

        tokio::spawn(async move {
            let mut reader = event::EventStream::new();
            let mut tick_interval = tokio::time::interval(tick_rate);

            loop {
                tokio::select! {
                    event = reader.next() => {
                        match event {
                            Some(Ok(CrosstermEvent::Key(key))) => {
                                if key.kind == KeyEventKind::Press {
                                    tx.send(TuiEvent::Key(key)).ok();
                                }
                            }
                            Some(Ok(CrosstermEvent::Resize(..))) => {
                                tx.send(TuiEvent::Resize).ok();
                            }
                            Some(Err(_)) | None => break,
                            _ => {}
                        }
                    }
                    _ = tick_interval.tick() => {
                        tx.send(TuiEvent::Tick).ok();
                    }
                }
            }
        });
    }

    pub fn draw<F>(&mut self, f: F) -> anyhow::Result<()>
    where
        F: FnOnce(&mut ratatui::Frame),
    {
        self.terminal.draw(f)?;
        Ok(())
    }
}
