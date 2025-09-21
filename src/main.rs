mod data;
mod http_client;
mod persistence;
mod ui;

fn main() -> iced::Result {
    ui::app::main()
}
