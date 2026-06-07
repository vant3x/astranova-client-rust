use crate::persistence::database::{
    Collection, CollectionFolder, CollectionRequest,
};
use iced::{
    widget::{button, column, container, row, scrollable, text, text_input},
    Alignment, Color, Element, Length, Renderer, Theme,
};

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub enum Message {
    ToggleExpanded(usize),
    SelectCollection(usize),
    SelectFolder(i32),
    LoadRequest(i32),
    NewCollectionNameChanged(String),
    CreateCollection,
    RenameCollection(usize, String),
    DeleteCollection(usize),
    NewFolderNameChanged(i32, String),
    CreateFolder(i32),
    RenameFolder(i32, String),
    DeleteFolder(i32),
    NewCollectionRequestName(String),
    SaveCurrentRequest,
    RenameRequest(i32, String),
    DeleteRequest(i32),
    Close,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum PanelState {
    #[default]
    Collections,
    CollectionDetail(usize),
    FolderDetail(usize, i32),
}

#[derive(Debug, Default)]
pub struct CollectionView {
    pub collections: Vec<Collection>,
    pub folders: Vec<CollectionFolder>,
    pub requests: Vec<CollectionRequest>,
    pub panel_state: PanelState,
    pub expanded_collections: Vec<bool>,
    pub expanded_folders: Vec<bool>,
    pub new_collection_name: String,
    pub new_folder_name: String,
    pub new_request_name: String,
    pub new_folder_target: Option<i32>,
    pub rename_collection_index: Option<usize>,
    pub rename_collection_value: String,
    pub rename_folder_id: Option<i32>,
    pub rename_folder_value: String,
    pub rename_request_id: Option<i32>,
    pub rename_request_value: String,
}

impl Clone for CollectionView {
    fn clone(&self) -> Self {
        Self {
            collections: self.collections.clone(),
            folders: self.folders.clone(),
            requests: self.requests.clone(),
            panel_state: self.panel_state.clone(),
            expanded_collections: self.expanded_collections.clone(),
            expanded_folders: self.expanded_folders.clone(),
            new_collection_name: self.new_collection_name.clone(),
            new_folder_name: self.new_folder_name.clone(),
            new_request_name: self.new_request_name.clone(),
            new_folder_target: self.new_folder_target,
            rename_collection_index: self.rename_collection_index,
            rename_collection_value: self.rename_collection_value.clone(),
            rename_folder_id: self.rename_folder_id,
            rename_folder_value: self.rename_folder_value.clone(),
            rename_request_id: self.rename_request_id,
            rename_request_value: self.rename_request_value.clone(),
        }
    }
}

impl CollectionView {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn update(&mut self, message: Message) -> Option<i32> {
        match message {
            Message::ToggleExpanded(idx) => {
                if let Some(expanded) = self.expanded_collections.get_mut(idx) {
                    *expanded = !*expanded;
                }
                None
            }
            Message::SelectCollection(idx) => {
                self.panel_state = PanelState::CollectionDetail(idx);
                None
            }
            Message::SelectFolder(folder_id) => {
                if let PanelState::CollectionDetail(col_idx) = self.panel_state {
                    self.panel_state = PanelState::FolderDetail(col_idx, folder_id);
                }
                None
            }
            Message::Close => {
                self.panel_state = PanelState::Collections;
                None
            }
            Message::NewCollectionNameChanged(name) => {
                self.new_collection_name = name;
                None
            }
            Message::CreateCollection => None,
            Message::NewFolderNameChanged(_col_id, name) => {
                self.new_folder_name = name;
                None
            }
            Message::CreateFolder(_col_id) => None,
            Message::DeleteCollection(idx) => {
                if idx < self.collections.len() {
                    self.collections.remove(idx);
                }
                None
            }
            Message::DeleteFolder(folder_id) => {
                self.folders.retain(|f| f.id != folder_id);
                None
            }
            Message::LoadRequest(req_id) => Some(req_id),
            Message::SaveCurrentRequest => None,
            _ => None,
        }
    }

    pub fn sync_collections(&mut self, collections: &[Collection]) {
        let old_len = self.expanded_collections.len();
        self.collections = collections.to_vec();
        if self.expanded_collections.len() < collections.len() {
            self.expanded_collections
                .resize(collections.len(), false);
        } else {
            self.expanded_collections.truncate(collections.len());
        }
        if old_len > collections.len() {
            if let PanelState::CollectionDetail(idx) = self.panel_state {
                if idx >= collections.len() {
                    self.panel_state = PanelState::Collections;
                }
            }
        }
    }

    pub fn sync_folders(&mut self, folders: &[CollectionFolder]) {
        self.folders = folders.to_vec();
        self.expanded_folders.resize(folders.len(), false);
    }

    pub fn sync_requests(&mut self, requests: &[CollectionRequest]) {
        self.requests = requests.to_vec();
    }

    pub fn view(&self) -> Element<'_, Message, Theme, Renderer> {
        match &self.panel_state {
            PanelState::Collections => self.collections_list_view(),
            PanelState::CollectionDetail(idx) => self.collection_detail_view(*idx),
            PanelState::FolderDetail(col_idx, folder_id) => {
                self.folder_detail_view(*col_idx, *folder_id)
            }
        }
    }

    fn collections_list_view(&self) -> Element<'_, Message, Theme, Renderer> {
        let header = row![
            text("Collections").size(16),
            button("+").on_press(Message::CreateCollection),
        ]
        .spacing(10)
        .align_y(Alignment::Center);

        let new_collection_input = text_input("New collection name...", &self.new_collection_name)
            .on_input(Message::NewCollectionNameChanged)
            .size(13)
            .padding(5);

        let mut list = column![].spacing(4);

        for (index, col) in self.collections.iter().enumerate() {
            let is_expanded = self.expanded_collections.get(index).copied().unwrap_or(false);
            let expand_icon = if is_expanded { "v " } else { "> " };

            let col_row = row![
                button(text(format!("{}{}", expand_icon, col.name)).size(13))
                    .on_press(Message::ToggleExpanded(index)),
                button(text("x").size(11).color(Color::from_rgb(0.8, 0.2, 0.2)))
                    .on_press(Message::DeleteCollection(index)),
            ]
            .spacing(4)
            .align_y(Alignment::Center);

            list = list.push(col_row);

            if is_expanded {
                let detail_button = button(
                    text(format!("    {} requests", self.requests.len()))
                        .size(11)
                        .color(Color::from_rgb(0.5, 0.5, 0.5)),
                )
                .on_press(Message::SelectCollection(index));

                list = list.push(detail_button);
            }
        }

        if self.collections.is_empty() {
            list = list.push(
                text("No collections yet.").size(13).color(Color::from_rgb(0.5, 0.5, 0.5)),
            );
        }

        container(
            column![
                header,
                new_collection_input,
                scrollable(list).height(Length::Fill),
            ]
            .spacing(8)
            .padding(10),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
    }

    fn collection_detail_view(&self, col_idx: usize) -> Element<'_, Message, Theme, Renderer> {
        let col = match self.collections.get(col_idx) {
            Some(c) => c,
            None => return self.collections_list_view(),
        };

        let back_button = button(text("< Back")).on_press(Message::Close);

        let header = row![
            back_button,
            text(&col.name).size(16),
        ]
        .spacing(10)
        .align_y(Alignment::Center);

        let new_folder_input = text_input("New folder name...", &self.new_folder_name)
            .on_input(|s| Message::NewFolderNameChanged(col.id, s))
            .size(13)
            .padding(5);

        let new_folder_button = button(text("+ Folder"))
            .on_press(Message::CreateFolder(col.id));

        let folder_controls = row![new_folder_input, new_folder_button].spacing(8);

        let mut list = column![].spacing(4);

        for (f_idx, folder) in self.folders.iter().enumerate() {
            let is_expanded = self.expanded_folders.get(f_idx).copied().unwrap_or(false);
            let expand_icon = if is_expanded { "v " } else { "> " };

            let folder_button = button(
                row![
                    text(format!("{}{}/", expand_icon, folder.name)).size(13),
                ]
                .spacing(4),
            )
            .on_press(Message::ToggleExpanded(col_idx));

            list = list.push(folder_button);

            if is_expanded {
                for req in &self.requests {
                    if req.folder_id == Some(folder.id) {
                        let method_color = method_color(&req.method);
                        let req_button = button(
                            row![
                                text(format!("    {}", req.method)).size(11).color(method_color),
                                text(&req.name).size(11),
                            ]
                            .spacing(6),
                        )
                        .on_press(Message::LoadRequest(req.id));
                        list = list.push(req_button);
                    }
                }

                let load_folder = button(text("      Open Folder"))
                    .on_press(Message::SelectFolder(folder.id));
                list = list.push(load_folder);
            }
        }

        let root_requests: Vec<&CollectionRequest> = self
            .requests
            .iter()
            .filter(|r| r.folder_id.is_none())
            .collect();

        if !root_requests.is_empty() {
            list = list.push(
                text("Requests:")
                    .size(12)
                    .color(Color::from_rgb(0.5, 0.5, 0.5)),
            );
        }

        for req in &root_requests {
            let method_color = method_color(&req.method);
            let url_short: String = req.url.chars().take(35).collect();
            let req_button = button(
                row![
                    text(&req.method).size(12).color(method_color),
                    text(url_short).size(12),
                ]
                .spacing(6),
            )
            .on_press(Message::LoadRequest(req.id));
            list = list.push(req_button);
        }

        container(
            column![
                header,
                folder_controls,
                scrollable(list).height(Length::Fill),
            ]
            .spacing(8)
            .padding(10),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
    }

    fn folder_detail_view(
        &self,
        _col_idx: usize,
        folder_id: i32,
    ) -> Element<'_, Message, Theme, Renderer> {
        let folder_name = self
            .folders
            .iter()
            .find(|f| f.id == folder_id)
            .map(|f| f.name.as_str())
            .unwrap_or("");

        let back_button = button(text("< Back")).on_press(Message::Close);

        let header = row![
            back_button,
            text(folder_name).size(16),
        ]
        .spacing(10)
        .align_y(Alignment::Center);

        let mut list = column![].spacing(4);

        for req in &self.requests {
            if req.folder_id == Some(folder_id) {
                let method_color = method_color(&req.method);
                let url_short: String = req.url.chars().take(35).collect();
                let req_button = button(
                    row![
                        text(&req.method).size(12).color(method_color),
                        text(&req.name).size(12),
                        text(url_short).size(11).color(Color::from_rgb(0.4, 0.4, 0.4)),
                    ]
                    .spacing(6),
                )
                .on_press(Message::LoadRequest(req.id));
                list = list.push(req_button);
            }
        }

        if self.requests.iter().all(|r| r.folder_id != Some(folder_id)) {
            list = list.push(
                text("No requests in this folder.")
                    .size(13)
                    .color(Color::from_rgb(0.5, 0.5, 0.5)),
            );
        }

        container(
            column![header, scrollable(list).height(Length::Fill)]
                .spacing(8)
                .padding(10),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
    }
}

fn method_color(method: &str) -> Color {
    match method {
        "GET" => Color::from_rgb(0.2, 0.7, 0.3),
        "POST" => Color::from_rgb(0.2, 0.4, 0.8),
        "PUT" => Color::from_rgb(0.8, 0.5, 0.1),
        "PATCH" => Color::from_rgb(0.8, 0.7, 0.1),
        "DELETE" => Color::from_rgb(0.8, 0.2, 0.2),
        "HEAD" => Color::from_rgb(0.5, 0.5, 0.5),
        "OPTIONS" => Color::from_rgb(0.6, 0.6, 0.6),
        _ => Color::from_rgb(0.5, 0.5, 0.5),
    }
}
