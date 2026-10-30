//! A conversation with one NPC, driven by the engine's `Conversation`.

use std::sync::Arc;

use engine::dialogue::{Choice, Conversation, DialogueBook, DialogueError};
use engine::game::Game;
use engine::shared::{DialogueId, VendorId};
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};

use crate::shared::input::Command;
use crate::shared::log::{Entry, Log};
use crate::shared::menu::{Item, Menu};
use crate::shared::theme;

/// What the player's command led to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Next {
    /// The conversation goes on.
    Stay,
    /// The conversation is over, or the player walked away.
    Leave,
    /// The topic opened this vendor's shop. Ask [`Talk::is_over`] whether the
    /// conversation is still there to return to.
    Shop(VendorId),
}

/// An open conversation and the menu of what the player can say.
#[derive(Debug)]
pub struct Talk {
    conversation: Conversation,
    speaker: String,
    choices: Vec<Choice>,
    menu: Menu,
    over: bool,
}

impl Talk {
    /// Opens the dialogue at its start topics.
    ///
    /// # Errors
    ///
    /// [`DialogueError`] when the book lacks the dialogue or it is not authored.
    pub fn open(
        book: Arc<DialogueBook>,
        dialogue: &DialogueId,
        speaker: String,
        game: &Game,
    ) -> Result<Self, DialogueError> {
        let conversation = Conversation::open(book, dialogue)?;
        let choices = conversation.choices(game);
        let menu = Menu::new(choices.len());
        Ok(Self {
            conversation,
            speaker,
            choices,
            menu,
            over: false,
        })
    }

    /// Applies a command: moves the cursor, says the selected topic, or walks away.
    /// What was said goes to the log; the game changes only if the topic's effects
    /// all apply.
    pub fn update(&mut self, command: Command, game: &mut Game, log: &mut Log) -> Next {
        if command == Command::Back {
            return Next::Leave;
        }
        let Some(choice) = self
            .menu
            .handle(command)
            .and_then(|index| self.choices.get(index))
            .cloned()
        else {
            return Next::Stay;
        };
        if !choice.unlocked {
            log.push(Entry::Note("You can't say that yet.".to_owned()));
            return Next::Stay;
        }
        match self.conversation.choose(game, &choice.key) {
            Ok(reply) => {
                log.push(Entry::You(choice.text));
                for line in reply.lines {
                    log.push(Entry::Said {
                        speaker: self.speaker.clone(),
                        text: line.text,
                    });
                }
                self.over = reply.over;
                if let Some(vendor) = reply.shop {
                    return Next::Shop(vendor);
                }
                if reply.over {
                    return Next::Leave;
                }
                self.refresh(game);
                Next::Stay
            }
            Err(error) => {
                log.push(Entry::Note(error.to_string()));
                Next::Stay
            }
        }
    }

    /// How many things the player can say right now.
    #[cfg(test)]
    pub fn choice_count(&self) -> usize {
        self.choices.len()
    }

    /// Whether the last topic closed the conversation.
    #[must_use]
    pub fn is_over(&self) -> bool {
        self.over
    }

    /// Rebuilds the menu, because what a topic did, or a trade, may unlock or hide
    /// others (some topics ask for caps).
    pub fn refresh(&mut self, game: &Game) {
        self.choices = self.conversation.choices(game);
        self.menu = Menu::new(self.choices.len());
    }

    /// Draws who is talking and what the player can say.
    pub fn render(&self, frame: &mut Frame, area: Rect) {
        let items: Vec<Item> = self
            .choices
            .iter()
            .map(|choice| {
                if choice.unlocked {
                    Item::new(choice.text.clone())
                } else {
                    Item::locked(choice.text.clone())
                }
            })
            .collect();
        let mut lines = vec![
            Line::styled(
                format!("You are talking to {}.", self.speaker),
                theme::text(),
            ),
            Line::default(),
            Line::styled("What do you say?", theme::heading()),
        ];
        lines.extend(self.menu.lines(&items));
        let block = Block::new()
            .borders(Borders::ALL)
            .border_style(theme::dim())
            .title(format!(" {} ", self.speaker));
        frame.render_widget(
            Paragraph::new(lines)
                .block(block)
                .wrap(Wrap { trim: false }),
            area,
        );
    }
}
