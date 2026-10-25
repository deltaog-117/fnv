//! What was said and done recently, bounded and word-wrapped to the pane.

use std::collections::VecDeque;

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};

use super::theme;

/// Older entries are dropped past this many, so a long session cannot grow the
/// log without bound.
const CAPACITY: usize = 200;

/// One thing worth remembering on screen.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Entry {
    /// Something an NPC said.
    Said {
        /// Who said it.
        speaker: String,
        /// The words.
        text: String,
    },
    /// What the player chose to say.
    You(String),
    /// A message from the game itself, such as a refusal.
    Note(String),
}

/// The recent history, oldest first.
#[derive(Debug, Default)]
pub struct Log {
    entries: VecDeque<Entry>,
}

impl Log {
    /// Adds an entry, dropping the oldest when the log is full.
    pub fn push(&mut self, entry: Entry) {
        if self.entries.len() == CAPACITY {
            self.entries.pop_front();
        }
        self.entries.push_back(entry);
    }

    /// The last `height` lines after wrapping every entry to `width` columns. The
    /// newest line is always the last one shown, which is why this wraps itself
    /// instead of leaving it to the paragraph widget.
    #[must_use]
    pub fn tail(&self, width: u16, height: u16) -> Vec<Line<'static>> {
        let width = usize::from(width).max(1);
        let mut lines: Vec<Line<'static>> = self
            .entries
            .iter()
            .rev()
            .flat_map(|entry| {
                let mut wrapped = wrap(entry, width);
                wrapped.reverse();
                wrapped
            })
            .take(usize::from(height))
            .collect();
        lines.reverse();
        lines
    }
}

fn wrap(entry: &Entry, width: usize) -> Vec<Line<'static>> {
    let (full, name, style) = match entry {
        Entry::Said { speaker, text } => (
            format!("{speaker}: {text}"),
            Some(format!("{speaker}: ")),
            theme::text(),
        ),
        Entry::You(text) => (format!("> {text}"), None, theme::dim()),
        Entry::Note(text) => (text.clone(), None, theme::warning()),
    };
    let options = textwrap::Options::new(width).subsequent_indent("  ");
    textwrap::wrap(&full, options)
        .into_iter()
        .enumerate()
        .map(|(index, row)| {
            let row = row.into_owned();
            match name.as_deref().filter(|_| index == 0) {
                Some(name) if row.starts_with(name) => Line::from(vec![
                    Span::styled(name.to_owned(), theme::heading()),
                    Span::styled(row[name.len()..].to_owned(), style),
                ]),
                _ => Line::styled(row, style),
            }
        })
        .collect()
}

/// Draws the log in a bordered pane.
pub fn render(frame: &mut Frame, area: Rect, log: &Log) {
    let block = Block::new()
        .borders(Borders::ALL)
        .border_style(theme::dim())
        .title(" Log ");
    let inner = block.inner(area);
    frame.render_widget(
        Paragraph::new(log.tail(inner.width, inner.height)).block(block),
        area,
    );
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::*;

    fn said(text: &str) -> Entry {
        Entry::Said {
            speaker: "Doc".to_owned(),
            text: text.to_owned(),
        }
    }

    fn plain(line: &Line<'_>) -> String {
        line.spans
            .iter()
            .map(|span| span.content.as_ref())
            .collect()
    }

    proptest! {
        #[test]
        fn the_log_never_holds_more_than_its_capacity(count in 0usize..500) {
            let mut log = Log::default();
            for n in 0..count {
                log.push(Entry::Note(n.to_string()));
            }
            prop_assert_eq!(log.entries.len(), count.min(CAPACITY));
        }

        #[test]
        // Words are shorter than the narrowest pane, because a longer word is split.
        fn a_tail_fits_the_pane_and_ends_with_the_newest_entry(
            words in proptest::collection::vec("[a-z]{1,7}", 1..40),
            width in 8u16..80, height in 1u16..12,
        ) {
            let mut log = Log::default();
            log.push(Entry::Note("older".to_owned()));
            log.push(Entry::Note(words.join(" ")));
            let tail = log.tail(width, height);
            prop_assert!(tail.len() <= usize::from(height));
            prop_assert!(tail.iter().all(|line| plain(line).chars().count() <= usize::from(width)));
            let last = words.last().expect("one to forty words");
            prop_assert!(plain(tail.last().expect("not empty")).ends_with(last.as_str()));
        }
    }

    #[test]
    fn the_speaker_is_styled_apart_from_the_words() {
        let mut log = Log::default();
        log.push(said("Hello there"));
        let lines = log.tail(40, 5);
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].spans.len(), 2);
        assert_eq!(plain(&lines[0]), "Doc: Hello there");
    }

    #[test]
    fn a_long_reply_wraps_with_a_hanging_indent() {
        let mut log = Log::default();
        log.push(said("one two three four five six seven eight"));
        let lines = log.tail(16, 10);
        assert!(lines.len() > 1);
        assert!(plain(&lines[1]).starts_with("  "));
    }
}
