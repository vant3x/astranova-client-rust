# AstraNova Client

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Commercial License](https://img.shields.io/badge/License-Commercial-green.svg)](LICENSING.md)

<img src="assets/astra-bg.png" alt="AstraNova Logo" width="300">

[Web](https://astranova-client.vercel.app/) -
[Download installer](https://astranova-client.vercel.app/) -

AstraNova Client is a desktop application built with Rust and Iced, designed to provide a user-friendly interface for making HTTP requests. It allows users to specify URLs, HTTP methods, headers, and request bodies, and then displays the response.

## License

This project is **dual-licensed**:

- **MIT** — Free for individual developers and small teams. See [LICENSE](LICENSE).
- **Commercial** — Required for enterprise features like team collections, cloud sync, and priority support. See [LICENSING.md](LICENSING.md).

## Getting Started

These instructions will get you a copy of the project up and running on your local machine for development and testing purposes.

### Prerequisites

You will need to have Rust and Cargo installed on your system. If you don't have them, you can install them using `rustup`:

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

### Running the Application

1.  **Clone the repository:**

    ```bash
    git clone https://github.com/vant3x/astranova-client-rust.git
    cd astranova-client
    ```

2.  **Run the application:**

    ```bash
    cargo run
    ```

    This command will compile the application and then run it. The application window should appear.

## macOS - conflicts

If you downloaded the app from GitHub Releases and macOS won't let you open it (due to an “unverified developer” error or because the app is corrupted), run the following in Terminal:



```bash
xattr -cr /Applications/AstraNova.app
```

This removes the “quarantine” label that macOS applies to apps downloaded from the web. You don't need to compile anything with `cargo`.

## Built With

*   [Rust](https://www.rust-lang.org/) - A language empowering everyone to build reliable and efficient software.
*   [Iced](https://iced.rs/) - A cross-platform GUI library for Rust, inspired by Elm.
