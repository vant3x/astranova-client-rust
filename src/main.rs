mod data;
mod error;
mod export;
mod http_client;
mod import;
mod persistence;
mod protocols;
mod services;
mod ui;
mod utils;

fn main() -> iced::Result {
    env_logger::init();
    ui::app::main()
}
