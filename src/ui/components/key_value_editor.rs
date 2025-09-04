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
        let entries_view = self.entries.iter().fold(column![], |col, entry| {
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
