# NextLine

A simple, GNOME-style task management app that feels like a text editor. Each line is a task.

## Features

- **Text editor feel** - Just type. Press Enter to create new tasks.
- **Project sidebar** - All your projects in one place
- **Progress tracking** - See done/total for each project
- **Three task states** - Pending, In Progress, and Done with emoji indicators
- **Keyboard-first** - Cycle task status with Ctrl+D
- **Offline** - Everything stored locally in plain text files
- **Native GNOME look** - Built with GTK4 and Libadwaita

## Installation

### Dependencies (Ubuntu 24.04)

```bash
sudo apt install libgtk-4-dev libadwaita-1-dev libgtksourceview-5-dev
```

### Build

```bash
cargo build --release
```

### Run

```bash
./target/release/nextline
```

### Install system-wide (optional)

```bash
sudo cp target/release/nextline /usr/local/bin/
sudo cp data/com.github.nextline.desktop /usr/share/applications/
```

## Usage

### Task Format

Tasks use emoji indicators for status:

| Emoji | Status | Description |
|-------|--------|-------------|
| ⬜ | Pending | Not started |
| 🟠 | In Progress | Currently working on |
| ✅ | Done | Completed |

Example:
```
⬜ Write documentation
🟠 Review pull request
✅ Fix login bug
```

### Keyboard Shortcuts

| Key | Action |
|-----|--------|
| `Ctrl+S` | Save |
| `Ctrl+Z` | Undo |
| `Ctrl+Shift+Z` | Redo |
| `Ctrl+D` | Cycle status: ⬜ → 🟠 → ✅ → ⬜ |
| `Ctrl+Enter` | New task |
| `Enter` | New task (when on a task line) |

### Projects

Projects are stored as text files in `~/Documents/NextLine/`. You can:
- Click the `+` button to create a new project
- Click a project in the sidebar to open it
- Use "Open Projects Folder" from the menu to view files directly

## File Format

Projects are plain text files (`.txt`, `.md`, or `.todo`). Example:

```
# My Project

⬜ First task
🟠 Task in progress
✅ Completed task
```

Also supports traditional markdown checkboxes (auto-converts to emoji):
```
- [ ] Pending task
- [x] Completed task
```

## License

GPL-3.0
