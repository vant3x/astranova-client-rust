use muda::accelerator::{Accelerator, Code, Modifiers, CMD_OR_CTRL};
use muda::{Menu, MenuEvent, MenuItem, PredefinedMenuItem, Submenu};

pub struct MenuIds {
    pub new_tab: muda::MenuId,
    pub save: muda::MenuId,
    pub find: muda::MenuId,
    pub toggle_sidebar: muda::MenuId,
    pub toggle_history: muda::MenuId,
    pub toggle_collections: muda::MenuId,
    pub toggle_dark_mode: muda::MenuId,
    pub cookie_manager: muda::MenuId,
    pub about: muda::MenuId,
    pub quit: muda::MenuId,
}

std::thread_local! {
    static MENU: std::cell::OnceCell<Menu> = const { std::cell::OnceCell::new() };
}

pub fn ids() -> &'static MenuIds {
    static IDS: std::sync::OnceLock<MenuIds> = std::sync::OnceLock::new();
    IDS.get_or_init(|| {
        let (ids, menu) = build_menu();
        MENU.with(|cell| {
            let _ = cell.set(menu);
        });
        ids
    })
}

fn acc(mods: Modifiers, key: Code) -> Accelerator {
    Accelerator::new(Some(mods), key)
}

fn build_menu() -> (MenuIds, Menu) {
    let menu = Menu::new();

    let new_tab = MenuItem::new("New Tab", true, Some(acc(CMD_OR_CTRL, Code::KeyT)));
    let open_collection = MenuItem::new(
        "Open Collection...",
        true,
        Some(acc(CMD_OR_CTRL, Code::KeyO)),
    );
    let save = MenuItem::new("Save", true, Some(acc(CMD_OR_CTRL, Code::KeyS)));
    let save_as = MenuItem::new(
        "Save to Collection...",
        true,
        Some(acc(CMD_OR_CTRL | Modifiers::SHIFT, Code::KeyS)),
    );
    let import_curl = MenuItem::new("Import cURL...", true, None);
    let import_postman = MenuItem::new("Import Postman Collection...", true, None);
    let import_openapi = MenuItem::new("Import OpenAPI/Swagger...", true, None);
    let export_postman = MenuItem::new("Export as Postman...", true, None);
    let export_har = MenuItem::new("Export as HAR...", true, None);
    let quit = MenuItem::new("Quit Astraio", true, Some(acc(CMD_OR_CTRL, Code::KeyQ)));

    let import_submenu = Submenu::new("Import", true);
    import_submenu
        .append_items(&[
            &import_curl as &dyn muda::IsMenuItem,
            &import_postman,
            &import_openapi,
        ])
        .expect("failed to append import submenu items");

    let export_submenu = Submenu::new("Export", true);
    export_submenu
        .append_items(&[&export_postman as &dyn muda::IsMenuItem, &export_har])
        .expect("failed to append export submenu items");

    let file_menu = Submenu::new("File", true);
    file_menu
        .append_items(&[
            &new_tab as &dyn muda::IsMenuItem,
            &PredefinedMenuItem::separator(),
            &open_collection,
            &PredefinedMenuItem::separator(),
            &save,
            &save_as,
            &PredefinedMenuItem::separator(),
            &import_submenu,
            &export_submenu,
            &PredefinedMenuItem::separator(),
            &quit,
        ])
        .expect("failed to append file menu items");

    let find = MenuItem::new("Find...", true, Some(acc(CMD_OR_CTRL, Code::KeyF)));

    let cut_item = MenuItem::new("Cut", true, None);
    let copy_item = MenuItem::new("Copy", true, None);
    let paste_item = MenuItem::new("Paste", true, None);
    let select_all_item = MenuItem::new("Select All", true, None);

    let edit_menu = Submenu::new("Edit", true);
    edit_menu
        .append_items(&[
            &cut_item as &dyn muda::IsMenuItem,
            &copy_item,
            &paste_item,
            &select_all_item,
            &PredefinedMenuItem::separator(),
            &find,
        ])
        .expect("failed to append edit menu items");

    let toggle_sidebar = MenuItem::new("Toggle Sidebar", true, Some(acc(CMD_OR_CTRL, Code::KeyB)));
    let toggle_history = MenuItem::new("Toggle History", true, Some(acc(CMD_OR_CTRL, Code::KeyH)));
    let toggle_collections = MenuItem::new(
        "Toggle Collections",
        true,
        Some(acc(CMD_OR_CTRL | Modifiers::SHIFT, Code::KeyC)),
    );
    let toggle_dark_mode =
        MenuItem::new("Toggle Dark Mode", true, Some(acc(CMD_OR_CTRL, Code::KeyD)));
    let cookie_manager = MenuItem::new(
        "Cookie Manager",
        true,
        Some(acc(CMD_OR_CTRL | Modifiers::SHIFT, Code::KeyK)),
    );
    let new_window = MenuItem::new(
        "New Window",
        true,
        Some(acc(CMD_OR_CTRL | Modifiers::SHIFT, Code::KeyN)),
    );

    let view_menu = Submenu::new("View", true);
    view_menu
        .append_items(&[
            &toggle_sidebar as &dyn muda::IsMenuItem,
            &toggle_history,
            &toggle_collections,
            &cookie_manager,
            &PredefinedMenuItem::separator(),
            &toggle_dark_mode,
            &PredefinedMenuItem::separator(),
            &new_window,
        ])
        .expect("failed to append view menu items");

    let about = MenuItem::new("About Astraio", true, None);

    let help_menu = Submenu::new("Help", true);
    help_menu
        .append_items(&[
            &about as &dyn muda::IsMenuItem,
            &PredefinedMenuItem::separator(),
            &PredefinedMenuItem::services(None),
        ])
        .expect("failed to append help menu items");

    menu.append_items(&[
        &file_menu as &dyn muda::IsMenuItem,
        &edit_menu,
        &view_menu,
        &help_menu,
    ])
    .expect("failed to append top-level menu items");

    #[cfg(target_os = "macos")]
    {
        let app_menu = Submenu::new("", true);
        app_menu
            .append_items(&[
                &PredefinedMenuItem::about(Some("About Astraio"), None) as &dyn muda::IsMenuItem,
                &PredefinedMenuItem::separator(),
                &PredefinedMenuItem::services(None),
                &PredefinedMenuItem::separator(),
                &PredefinedMenuItem::hide(Some("Hide Astraio")),
                &PredefinedMenuItem::hide_others(Some("Hide Others")),
                &PredefinedMenuItem::show_all(Some("Show All")),
                &PredefinedMenuItem::separator(),
            ])
            .expect("failed to append macOS app menu items");
        menu.prepend(&app_menu).expect("failed to prepend macOS app menu");
    }

    let menu_ids = MenuIds {
        new_tab: new_tab.id().clone(),
        save: save.id().clone(),
        find: find.id().clone(),
        toggle_sidebar: toggle_sidebar.id().clone(),
        toggle_history: toggle_history.id().clone(),
        toggle_collections: toggle_collections.id().clone(),
        toggle_dark_mode: toggle_dark_mode.id().clone(),
        cookie_manager: cookie_manager.id().clone(),
        about: about.id().clone(),
        quit: quit.id().clone(),
    };

    (menu_ids, menu)
}

#[cfg(target_os = "macos")]
pub fn attach_macos() {
    ids();
    MENU.with(|cell| {
        if let Some(menu) = cell.get() {
            menu.init_for_nsapp();
        }
    });
}

#[cfg(target_os = "windows")]
pub fn attach_windows(hwnd: isize) {
    ids();
    MENU.with(|cell| {
        if let Some(menu) = cell.get() {
            unsafe {
                let _ = menu.init_for_hwnd(hwnd);
            }
        }
    });
}

pub fn handle_menu_event(event: &MenuEvent) -> Option<super::app::Message> {
    use super::app::Message;

    let i = ids();

    if event.id == i.new_tab {
        Some(Message::AddRequestTab)
    } else if event.id == i.save {
        Some(Message::HttpRequestViewMsg(
            0,
            super::views::http_request_view::Message::SaveScripts,
        ))
    } else if event.id == i.find {
        Some(Message::ToggleResponseSearch)
    } else if event.id == i.toggle_sidebar {
        Some(Message::ToggleSidebar)
    } else if event.id == i.toggle_history {
        Some(Message::ToggleHistory)
    } else if event.id == i.toggle_collections {
        Some(Message::ToggleCollections)
    } else if event.id == i.toggle_dark_mode {
        Some(Message::ToggleTheme)
    } else if event.id == i.cookie_manager {
        Some(Message::ToggleCookieManager)
    } else if event.id == i.about {
        Some(Message::ShowAbout)
    } else if event.id == i.quit {
        Some(Message::Quit)
    } else {
        None
    }
}
