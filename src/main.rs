mod data;
mod error;
mod http_client;
mod persistence;
mod protocols;
mod ui;

fn main() -> iced::Result {
    env_logger::init();
    ui::app::main()
}
