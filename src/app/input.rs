// Key event handling: maps key presses to actions.

use crate::action::Action;
use crate::app::App;
use crate::components::Component;
use crossterm::event::{KeyCode, KeyEvent};

impl App {
    pub fn handle_key(&mut self, key: KeyEvent) -> anyhow::Result<()> {
        use KeyCode::*;

        // Overlays consume all keys
        if self.show_help {
            self.action_tx.send(Action::HideHelp)?;
            return Ok(());
        }
        if self.direct_play_modal.is_visible() {
            self.direct_play_modal.handle_key_event(key)?;
            return Ok(());
        }

        // Keys that work regardless of search focus
        match key.code {
            Tab => {
                return self
                    .action_tx
                    .send(Action::SwitchSubTab((self.nts_tab.active_index() + 1) % 3))
                    .map_err(Into::into)
            }
            BackTab => {
                return self
                    .action_tx
                    .send(Action::SwitchSubTab((self.nts_tab.active_index() + 2) % 3))
                    .map_err(Into::into)
            }
            Esc => return self.action_tx.send(Action::Back).map_err(Into::into),
            _ => {}
        }

        // In search mode, forward everything else to the search bar
        if self.search_bar.is_focused() {
            self.search_bar.handle_key_event(key)?;
            return Ok(());
        }

        // Normal-mode keybindings
        match key.code {
            Char('q') => self.action_tx.send(Action::Quit)?,
            Char('?') => self.action_tx.send(Action::ShowHelp)?,
            Char('o') => self.action_tx.send(Action::OpenDirectPlay)?,
            Char(' ') => self.action_tx.send(Action::TogglePlayPause)?,
            Char('n') => self.action_tx.send(Action::NextTrack)?,
            Char('p') => self.action_tx.send(Action::PrevTrack)?,
            Char('s') => self.action_tx.send(Action::Stop)?,
            Char('/') => self.action_tx.send(Action::FocusSearch)?,
            Char('c') => self.action_tx.send(Action::ClearQueue)?,
            Char(']') => self.action_tx.send(Action::VolumeUp)?,
            Char('[') => self.action_tx.send(Action::VolumeDown)?,
            Char('a') => {
                if let Some(item) = self.discovery_list.selected_item() {
                    self.action_tx.send(Action::AddToQueue(item.clone()))?;
                }
            }
            Char('A') => {
                if let Some(item) = self.discovery_list.selected_item() {
                    self.action_tx.send(Action::AddToQueueNext(item.clone()))?;
                }
            }
            Char('r') if self.error_message.is_some() => {
                self.action_tx.send(Action::LoadNtsLive)?;
                self.error_message = None;
            }
            Char(c) if c.is_ascii_digit() => {
                let idx = c.to_digit(10).unwrap_or(0) as usize;
                if (1..=3).contains(&idx) {
                    self.action_tx.send(Action::SwitchSubTab(idx - 1))?;
                }
            }
            _ => {
                self.discovery_list.handle_key_event(key)?;
            }
        }
        Ok(())
    }
}
