//! Event handling for TUI

use crossterm::event::{self, Event as CrosstermEvent, KeyEvent, KeyEventKind, MouseEvent};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

/// Terminal events
#[allow(dead_code)]
pub enum Event {
    /// Key press event
    Key(KeyEvent),
    /// Mouse event
    Mouse(MouseEvent),
    /// Terminal resize
    Resize(u16, u16),
    /// Tick event (for periodic updates)
    Tick,
}

/// Event handler that spawns a thread to poll for events
pub struct EventHandler {
    rx: mpsc::Receiver<Event>,
}

impl EventHandler {
    /// Create a new event handler with the specified tick rate
    pub fn new(tick_rate: Duration) -> Self {
        let (tx, rx) = mpsc::channel();

        // Clone tx for the tick thread
        let tick_tx = tx.clone();

        thread::spawn(move || {
            loop {
                // Poll for terminal events
                if event::poll(Duration::from_millis(100)).is_ok() {
                    if let Ok(crossterm_event) = event::read() {
                        match crossterm_event {
                            CrosstermEvent::Key(key) => {
                                // Only send key press events (ignore release/repeat)
                                if key.kind == KeyEventKind::Press {
                                    if tx.send(Event::Key(key)).is_err() {
                                        break;
                                    }
                                }
                            }
                            CrosstermEvent::Mouse(mouse) => {
                                if tx.send(Event::Mouse(mouse)).is_err() {
                                    break;
                                }
                            }
                            CrosstermEvent::Resize(width, height) => {
                                if tx.send(Event::Resize(width, height)).is_err() {
                                    break;
                                }
                            }
                            // Ignore focus and paste events
                            CrosstermEvent::FocusGained
                            | CrosstermEvent::FocusLost
                            | CrosstermEvent::Paste(_) => {}
                        }
                    }
                }
            }
        });

        // Spawn tick thread
        thread::spawn(move || {
            loop {
                if tick_tx.send(Event::Tick).is_err() {
                    break;
                }
                thread::sleep(tick_rate);
            }
        });

        Self { rx }
    }

    /// Receive the next event (blocking)
    pub fn recv(&self) -> Result<Event, mpsc::RecvError> {
        self.rx.recv()
    }

    /// Try to receive an event (non-blocking)
    #[allow(dead_code)]
    pub fn try_recv(&self) -> Result<Event, mpsc::TryRecvError> {
        self.rx.try_recv()
    }
}