fn main() {
    // On Windows: embed the app icon into the .exe resource section.
    // This makes the executable show the custom icon in Explorer/taskbar/title bar.
    // On macOS and Linux this block is compiled out and does nothing.
    #[cfg(target_os = "windows")]
    {
        let mut res = winresource::WindowsResource::new();
        res.set_icon("assets/icon.ico");
        res.compile().expect("Failed to compile Windows resources");
    }
}
