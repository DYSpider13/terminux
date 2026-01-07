# Terminux

A modern Linux SSH terminal manager inspired by MobaXterm, built with GTK4 and Rust.

## Features

- **SSH Session Management** - Create, save, and organize SSH connections
- **Multi-tab Interface** - Work with multiple terminals simultaneously
- **Local Terminal** - Built-in local shell terminal
- **SSH Key Authentication** - Support for password and SSH key authentication
- **Session Persistence** - Sessions are saved locally and persist across restarts
- **SFTP File Browser** - Browse remote files (sidebar integration)
- **Modern UI** - Native GTK4/libadwaita interface following GNOME HIG

## Screenshots

*Coming soon*

## Installation

### Dependencies

Terminux requires the following system libraries:

- GTK4 (>= 4.0)
- libadwaita (>= 1.0)
- VTE4 (>= 0.70)
- SQLite3

#### Fedora/RHEL

```bash
sudo dnf install gtk4-devel libadwaita-devel vte291-gtk4-devel sqlite-devel
```

#### Ubuntu/Debian

```bash
sudo apt install libgtk-4-dev libadwaita-1-dev libvte-2.91-gtk4-dev libsqlite3-dev
```

#### Arch Linux

```bash
sudo pacman -S gtk4 libadwaita vte4 sqlite
```

### Building from Source

1. **Install Rust** (if not already installed):
   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   ```

2. **Clone the repository**:
   ```bash
   git clone https://github.com/DYspider13/terminux.git
   cd terminux
   ```

3. **Build the project**:
   ```bash
   cargo build --release
   ```

4. **Run**:
   ```bash
   ./target/release/terminux
   ```

## Usage

### Creating a New SSH Session

1. Click the **+** button in the header bar or press `Ctrl+Shift+N`
2. Fill in the connection details:
   - Session name
   - Host/IP address
   - Port (default: 22)
   - Username
3. Choose authentication method:
   - **Password**: Enter your password
   - **SSH Key**: Select your private key file
4. Click **Save & Connect**

### Keyboard Shortcuts

| Action | Shortcut |
|--------|----------|
| New Session | `Ctrl+Shift+N` |
| New Local Tab | `Ctrl+T` |
| Close Tab | `Ctrl+W` |
| Quit | `Ctrl+Q` |

### Connecting to Saved Sessions

Double-click on any session in the sidebar to connect.

## Data Storage

- **Database**: `~/.local/share/terminux/sessions.db`
- Sessions are stored locally using SQLite

## Roadmap

- [ ] Session folders/groups
- [ ] Password storage in system keyring
- [ ] Session import/export
- [ ] SFTP file transfers
- [ ] Split terminal panes
- [ ] SSH tunneling UI
- [ ] Session search
- [ ] Custom color schemes

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

1. Fork the repository
2. Create your feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add some amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

## License

This project is licensed under the GPL-3.0 License - see the [LICENSE](LICENSE) file for details.

## Author

**Younes Khadraoui** - [@DYspider13](https://github.com/DYspider13)

## Acknowledgments

- Inspired by [MobaXterm](https://mobaxterm.mobatek.net/)
- Built with [GTK4](https://gtk.org/) and [libadwaita](https://gnome.pages.gitlab.gnome.org/libadwaita/)
- SSH functionality powered by [russh](https://github.com/warp-tech/russh)
