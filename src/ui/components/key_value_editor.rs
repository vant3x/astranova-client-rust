use iced::{
    widget::{button, column, row, text, text_input},
    Element,
};

#[derive(Debug, Clone)]
pub struct KeyValueEntry {
    pub id: usize,
    pub key: String,
    pub value: String,
}

impl KeyValueEntry {
    fn new(id: usize) -> Self {
        Self {
            id,
            key: String::new(),
            value: String::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct KeyValueEditor {
    pub entries: Vec<KeyValueEntry>,
    next_id: usize,
    button_text: String,
}

#[derive(Debug, Clone)]
pub enum Message {
    EntryKeyChanged(usize, String),
    EntryValueChanged(usize, String),
    AddEntry,
    RemoveEntry(usize),
}

impl Default for KeyValueEditor {
    fn default() -> Self {
        Self::new("Add Entry".to_string())
    }
}

impl KeyValueEditor {
    pub fn new(button_text: String) -> Self {
        Self {
            entries: vec![KeyValueEntry::new(0)],
            next_id: 1,
            button_text,
        }
    }
    pub fn update(&mut self, message: Message) {
        match message {
            Message::EntryKeyChanged(id, new_key) => {
                if let Some(entry) = self.entries.iter_mut().find(|e| e.id == id) {
                    entry.key = new_key;
                }
            }
            Message::EntryValueChanged(id, new_value) => {
                if let Some(entry) = self.entries.iter_mut().find(|e| e.id == id) {
                    entry.value = new_value;
                }
            }
            Message::AddEntry => {
                self.entries.push(KeyValueEntry::new(self.next_id));
                self.next_id += 1;
            }
            Message::RemoveEntry(id) => {
                self.entries.retain(|entry| entry.id != id);
            }
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        let entries_view = self.entries.iter().fold(column![].spacing(8), |col, entry| {
            col.push(
                row![
                    text_input("Key", &entry.key)
                        .on_input(move |k| Message::EntryKeyChanged(entry.id, k)),
                    text_input("Value", &entry.value)
                        .on_input(move |v| Message::EntryValueChanged(entry.id, v)),
                    button(text("Remove")).on_press(Message::RemoveEntry(entry.id))
                ]
                .spacing(10),
            )
        });

        column![
            entries_view,
            button(text(&self.button_text)).on_press(Message::AddEntry)
        ]
        .spacing(10)
        .into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_editor_has_one_empty_entry() {
        let editor = KeyValueEditor::default();
        assert_eq!(editor.entries.len(), 1);
        assert!(editor.entries[0].key.is_empty());
        assert!(editor.entries[0].value.is_empty());
    }

    #[test]
    fn add_entry_increases_count() {
        let mut editor = KeyValueEditor::default();
        editor.update(Message::AddEntry);
        assert_eq!(editor.entries.len(), 2);
        editor.update(Message::AddEntry);
        assert_eq!(editor.entries.len(), 3);
    }

    #[test]
    fn add_entry_assigns_unique_ids() {
        let mut editor = KeyValueEditor::default();
        editor.update(Message::AddEntry);
        editor.update(Message::AddEntry);
        let ids: Vec<usize> = editor.entries.iter().map(|e| e.id).collect();
        assert_eq!(ids, vec![0, 1, 2]);
    }

    #[test]
    fn remove_entry_decreases_count() {
        let mut editor = KeyValueEditor::default();
        editor.update(Message::AddEntry);
        assert_eq!(editor.entries.len(), 2);
        editor.update(Message::RemoveEntry(0));
        assert_eq!(editor.entries.len(), 1);
    }

    #[test]
    fn remove_entry_by_id() {
        let mut editor = KeyValueEditor::default();
        editor.update(Message::AddEntry);
        editor.update(Message::AddEntry);
        editor.update(Message::RemoveEntry(1));
        let ids: Vec<usize> = editor.entries.iter().map(|e| e.id).collect();
        assert_eq!(ids, vec![0, 2]);
    }

    #[test]
    fn change_entry_key() {
        let mut editor = KeyValueEditor::default();
        editor.update(Message::EntryKeyChanged(0, "Content-Type".to_string()));
        assert_eq!(editor.entries[0].key, "Content-Type");
    }

    #[test]
    fn change_entry_value() {
        let mut editor = KeyValueEditor::default();
        editor.update(Message::EntryValueChanged(
            0,
            "application/json".to_string(),
        ));
        assert_eq!(editor.entries[0].value, "application/json");
    }

    #[test]
    fn change_key_on_nonexistent_entry_does_not_panic() {
        let mut editor = KeyValueEditor::default();
        editor.update(Message::EntryKeyChanged(999, "key".to_string()));
        // Should not panic, just do nothing
    }

    #[test]
    fn remove_nonexistent_entry_does_not_panic() {
        let mut editor = KeyValueEditor::default();
        editor.update(Message::RemoveEntry(999));
        assert_eq!(editor.entries.len(), 1);
    }

    #[test]
    fn custom_button_text() {
        let editor = KeyValueEditor::new("Add Header".to_string());
        assert_eq!(editor.button_text, "Add Header");
    }
}
