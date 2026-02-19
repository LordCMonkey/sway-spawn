# Spawn

Spawn is a lightweight utility for the [Sway](https://swaywm.org/) tiling window manager. It allows you to toggle specific applications to and from the scratchpad with a single keybinding, effectively creating "drop-down" terminals or quick-access windows for any program.

## Features

- **Toggle Visibility**: Instantly show or hide specific applications.
- **Auto-Start**: Automatically launches the application if it's not running.
- **Smart Focus**: Brings the window to focus if it's visible but not active.
- **Flexible Configuration**: Supports matching by Title, AppId, or Class.

## Getting Started

Follow these steps to get a drop-down terminal up and running quickly.

### 1. Installation

Build the project from source:

```bash
cargo build --release
```
And move the build binary somwehere in your `$PATH`, for example:
```bash
cp target/release/spawn ~/.local/bin/
```

### 2. Quick Configuration

Create the configuration file at `~/.config/spawn/spawn.toml`:

```toml
# Set your preferred terminal emulator (e.g., alacritty, kitty, foot)
terminal = "alacritty"

# Define a 'drop-down' terminal app named 'console'
[apps.console]
command = "zsh"           # Command to run inside the terminal
is_terminal = true         # It's a terminal app
identifier = { Title = "Dropdown_Console" } # To use as terminal title and sway window identifier
```

### 3. Sway Integration

Add the following to your Sway config (usually `~/.config/sway/config`):

```sway
# define the spawn command
set $spawn ~/.local/bin/spawn

# Configure the window to float and be positioned in the center
for_window [title="Dropdown_Console"] floating enable
for_window [title="Dropdown_Console"] move position center
for_window [title="Dropdown_Console"] resize set 800 600

# Bind a key to toggle the floating console (e.g., Mod4+d)
bindsym Mod4+d exec $spawn console
```

Reload Sway and press `$mod+d`. You now have a togglable terminal handy regardless of
workspace!

## Usage

Once configured, the behavior is simple:

1.  **Press Key**:
    *   **If not running**: Spawn launches the application.
    *   **If running & hidden**: Spawn moves it from the scratchpad to the current workspace.
    *   **If visible & focused**: Spawn moves it to the scratchpad (hides it).
    *   **If visible & unfocused**: Spawn focuses the window. This may change your workspace.

## Configuration

### spawn.toml

The configuration file is located at `~/.config/spawn/spawn.toml`.

#### Global Settings

*   `terminal`: (String) The terminal emulator command used for terminal applications. Spawn currently assumes the terminal accepts `--title <TITLE> --command <CMD>` arguments (common in alacritty, kitty, etc.).

#### Application Definitions (`[apps.<name>]`)

Each section `[apps.<name>]` defines a new togglable application.

*   `command`: (String) The command to execute. For terminal apps, this is the shell or program running inside. for GUI apps, this is the binary name.
*   `is_terminal`: (Boolean) `true` if it runs inside a terminal, `false` for GUI applications.
*   `identifier`: (Object) How Spawn finds the window in Sway's tree. Options:
    *   `{ Title = "..." }`: Match exact window title.
    *   `{ AppId = "..." }`: Match Wayland App ID.
    *   `{ Class = "..." }`: Match window class (XWayland).
*   `startup_override`: (Optional String) Fully custom command to launch the application if the default terminal command construction doesn't work for you.

**Example `spawn.toml`:**

```toml
terminal = "alacritty"

# Terminal App
[apps.python]
command = "python3"
is_terminal = true
identifier = { Title = "Dropdown_Python" }

# GUI App
[apps.obsidian]
command = "obsidian"
is_terminal = false
identifier = { AppId = "obsidian" }
```

### Sway Configuration

You need to tell Sway how to handle these windows. Generally, I like them floating.

```sway
# General rule for all spawn windows starting with "Dropdown_"
for_window [title="^Dropdown_"] floating enable
for_window [title="^Dropdown_"] move position center

# Specific rule for a GUI app
for_window [app_id="obsidian"] floating enable

# Keybindings
bindsym $mod+p exec spawn python
bindsym $mod+o exec spawn obsidian
```
