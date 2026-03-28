# sliders_popup

Lightweight Wayland popup with JSON-configured sliders, switches, and buttons. Written in Rust with GTK3 + gtk-layer-shell. CSS themeable.

## Install

### Nix (flake)

```bash
nix run github:LillHill/sliders_popup -- < config.json

# or add to your flake inputs
nix profile install github:LillHill/sliders_popup
```

### Build from source

```bash
# Dependencies (Debian/Ubuntu)
sudo apt install libgtk-3-dev libgtk-layer-shell-dev cargo

# Build
cargo build --release
# Binary at ./target/release/sliders_popup
```

## Usage

Pipe JSON config to stdin:

```bash
sliders_popup < config.json
cat config.json | sliders_popup
sliders_popup --css /path/to/style.css < config.json
```

## JSON Config

### Minimal example

```json
{
  "children": [
    { "type": "slider", "name": "volume", "label": "Volume",
      "cmd": "wpctl set-volume @DEFAULT_AUDIO_SINK@ ${VAL}%",
      "min": 0, "max": 100, "step": 1,
      "read_cmd": "wpctl get-volume @DEFAULT_AUDIO_SINK@ | awk '{printf \"%.0f\", $2 * 100}'" }
  ]
}
```

### Full example

```json
{
  "width": 350,
  "anchor": "top right",
  "margin": { "top": 10, "right": 10 },
  "layer": "overlay",
  "close_on_focus_loss": true,
  "css": "themes/catppuccin.css",
  "orientation": "vertical",
  "spacing": 6,
  "children": [
    { "type": "slider", "name": "volume", "label": "Volume",
      "cmd": "wpctl set-volume @DEFAULT_AUDIO_SINK@ ${VAL}%",
      "read_cmd": "wpctl get-volume @DEFAULT_AUDIO_SINK@ | awk '{printf \"%.0f\", $2 * 100}'",
      "min": 0, "max": 150, "step": 1 },
    { "type": "slider", "name": "brightness", "label": "Brightness",
      "cmd": "brightnessctl set ${VAL}%",
      "read_cmd": "brightnessctl -m | awk -F, '{print int($4)}'",
      "min": 1, "max": 100, "step": 1 },
    { "type": "switch", "name": "mute", "label": "Mute",
      "cmd": "wpctl set-mute @DEFAULT_AUDIO_SINK@ toggle",
      "read_cmd": "wpctl get-volume @DEFAULT_AUDIO_SINK@ | grep -q MUTED && echo 0 || echo 1" },
    { "type": "box", "orientation": "horizontal", "spacing": 8, "children": [
      { "type": "button", "name": "lock", "label": "Lock", "cmd": "swaylock" },
      { "type": "button", "name": "power", "label": "Power", "cmd": "wlogout" }
    ]}
  ]
}
```

## Widget types

### Slider

Runs `cmd` via `sh -c` with `$VAL` set to the current value on every change.

```json
{ "type": "slider", "name": "volume", "label": "Volume",
  "cmd": "wpctl set-volume @DEFAULT_AUDIO_SINK@ ${VAL}%",
  "read_cmd": "wpctl get-volume @DEFAULT_AUDIO_SINK@ | awk '{printf \"%.0f\", $2 * 100}'",
  "min": 0, "max": 100, "step": 1, "value": 50,
  "orientation": "horizontal", "length": 200,
  "show_value": true, "value_pos": "top", "digits": 0 }
```

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `name` | string | `"widget"` | Widget name for CSS (`#name`) |
| `label` | string | null | Label text above the slider |
| `cmd` | string | `""` | Command to run — `$VAL` is set to value |
| `read_cmd` | string | null | Command to read initial value (stdout parsed as number) |
| `min` | number | 0 | Minimum |
| `max` | number | 100 | Maximum |
| `step` | number | 1 | Increment |
| `value` | number | min | Fallback initial value (if `read_cmd` absent/fails) |
| `orientation` | string | `"horizontal"` | `"horizontal"` or `"vertical"` |
| `length` | int | 200 (v) / auto (h) | Size in pixels along slider axis |
| `show_value` | bool | true | Show numeric value |
| `value_pos` | string | `"top"` | `"top"`, `"bottom"`, `"left"`, `"right"` |
| `digits` | int | auto | Decimal places (auto: 0 for int step, 2 otherwise) |

### Button

Fires a command on click. No `$VAL`.

```json
{ "type": "button", "name": "lock", "label": "Lock Screen", "cmd": "swaylock" }
```

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `name` | string | `"widget"` | Widget name for CSS |
| `label` | string | name | Button text |
| `cmd` | string | `""` | Command to run on click |

### Switch

Stateful toggle. Supports three command modes:

```json
// Separate on/off commands (cleanest)
{ "type": "switch", "name": "wifi", "label": "Wi-Fi",
  "cmd_on": "nmcli radio wifi on",
  "cmd_off": "nmcli radio wifi off",
  "read_cmd": "nmcli radio wifi | grep -q enabled && echo 1 || echo 0" }

// Single toggle command (for self-toggling tools)
{ "type": "switch", "name": "mute", "label": "Mute",
  "cmd": "wpctl set-mute @DEFAULT_AUDIO_SINK@ toggle",
  "read_cmd": "wpctl get-volume @DEFAULT_AUDIO_SINK@ | grep -q MUTED && echo 1 || echo 0" }

// Single command with $VAL branching
{ "type": "switch", "name": "dnd", "label": "Do Not Disturb",
  "cmd": "sh -c 'if [ \"$VAL\" = 1 ]; then makoctl mode -a dnd; else makoctl mode -r dnd; fi'" }
```

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `name` | string | `"widget"` | Widget name for CSS |
| `label` | string | null | Label text (left-aligned) |
| `cmd` | string | `""` | Fallback command — `$VAL` is `"1"` or `"0"` |
| `cmd_on` | string | null | Command to run when toggled ON (overrides `cmd`) |
| `cmd_off` | string | null | Command to run when toggled OFF (overrides `cmd`) |
| `read_cmd` | string | null | Command to read state (`1`/`true`/`yes`/`on` = active) |
| `value` | bool | false | Fallback initial state |

**Switch vs Button**: Button is fire-and-forget with no state. Switch tracks visual on/off state, can read initial state via `read_cmd`, and runs distinct commands per state.

### Box

Nestable container for complex layouts.

```json
{ "type": "box", "orientation": "horizontal", "spacing": 8, "homogeneous": true,
  "children": [ ... ] }
```

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `orientation` | string | `"horizontal"` | `"horizontal"` or `"vertical"` |
| `name` | string | null | Widget name for CSS |
| `spacing` | int | 4 | Gap between children |
| `homogeneous` | bool | false | Equal-size children |
| `children` | array | `[]` | Nested widgets |

## Top-level options

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `width` | int | 300 | Window width |
| `height` | int | -1 | Window height (-1 = auto) |
| `anchor` | string | `"right"` | `"top"`, `"bottom"`, `"left"`, `"right"`, `"top right"`, `"center"`, etc. |
| `margin` | object/int | 0 | Edge margins `{ "top": 10, "right": 10 }` or uniform int |
| `layer` | string | `"overlay"` | `"background"`, `"bottom"`, `"top"`, `"overlay"` |
| `namespace` | string | `"sliders_popup"` | Layer shell namespace / CSS window name |
| `close_on_focus_loss` | bool | true | Close when focus is lost |
| `css` | string | null | Path to CSS file |
| `css_inline` | string | null | Inline CSS string |
| `orientation` | string | `"vertical"` | Root container direction |
| `spacing` | int | 4 | Root container spacing |
| `exclusive_zone` | int | -1 | Layer shell exclusive zone |
| `children` | array | `[]` | Widget tree (preferred) |
| `sliders` | array | `[]` | Flat slider list (backward compat) |

## CSS Themes

Six themes included in `themes/`:

| Theme | File | Description |
|-------|------|-------------|
| Catppuccin | `themes/catppuccin.css` | Dark purple/blue (Mocha) |
| Nord | `themes/nord.css` | Arctic blue/teal |
| Gruvbox | `themes/gruvbox.css` | Warm retro brown/green |
| Rose Pine | `themes/rose_pine.css` | Soft purple on dark |
| Transparent | `themes/transparent.css` | Glassmorphism |
| Minimal | `themes/minimal.css` | Subtle tweaks on stock GTK |

Compare them quickly:

```bash
sliders_popup < examples/themes/default.json      # no CSS
sliders_popup < examples/themes/catppuccin.json
sliders_popup < examples/themes/nord.json
sliders_popup < examples/themes/gruvbox.json
sliders_popup < examples/themes/rose_pine.json
sliders_popup < examples/themes/transparent.json
sliders_popup < examples/themes/minimal.json
```

### CSS selectors

| Selector | Target |
|----------|--------|
| `#sliders_popup` | Window (or your `namespace`) |
| `#sliders_box` | Root container |
| `.slider-row` | Slider wrapper box |
| `.slider-label` | Slider label |
| `.slider` / `#<name>` | Scale widget |
| `.popup-button` | Button |
| `.switch-row` | Switch wrapper box |
| `.switch-label` | Switch label |
| `.popup-switch` | Switch widget |
| `.container` | Nested box |

## NixOS / Home Manager

### Add to flake inputs

```nix
{
  inputs.sliders_popup.url = "github:LillHill/sliders_popup";
}
```

### Home Manager module

```nix
{ inputs, pkgs, config, ... }:
{
  imports = [ inputs.sliders_popup.homeManagerModules.sliders_popup ];

  programs.sliders_popup = {
    enable = true;
    package = inputs.sliders_popup.packages.${pkgs.system}.default;

    popups = {
      audio = {
        settings = {
          width = 350;
          anchor = "top right";
          margin = { top = 10; right = 10; };
          children = [
            { type = "slider"; name = "volume"; label = "Volume";
              cmd = "wpctl set-volume @DEFAULT_AUDIO_SINK@ \${VAL}%";
              read_cmd = "wpctl get-volume @DEFAULT_AUDIO_SINK@ | awk '{printf \"%.0f\", $2 * 100}'";
              min = 0; max = 100; step = 1; }
            { type = "switch"; name = "mute"; label = "Mute";
              cmd = "wpctl set-mute @DEFAULT_AUDIO_SINK@ toggle"; }
          ];
        };
        css = ./themes/catppuccin.css;  # optional CSS override
      };

      toggles = {
        settings = {
          width = 300;
          anchor = "top right";
          children = [
            { type = "switch"; name = "wifi"; label = "Wi-Fi";
              cmd_on = "nmcli radio wifi on";
              cmd_off = "nmcli radio wifi off";
              read_cmd = "nmcli radio wifi | grep -q enabled && echo 1 || echo 0"; }
            { type = "button"; name = "lock"; label = "Lock";
              cmd = "swaylock"; }
          ];
        };
      };
    };
  };
}
```

### Run from shell

```bash
# Each popup is a script in PATH
sliders_popup-audio
sliders_popup-toggles
```

### Keybinds (Hyprland)

```nix
wayland.windowManager.hyprland.settings.bind = [
  "$mod, V, exec, sliders_popup-audio"
  "$mod, T, exec, sliders_popup-toggles"
  # or use the full path:
  "$mod, V, exec, ${config.programs.sliders_popup.popups.audio.command}"
];
```

### One popup opening another

```nix
{ type = "button"; name = "toggles"; label = "Toggles";
  cmd = config.programs.sliders_popup.popups.toggles.command; }
```

## Examples

See `examples/` for ready-to-use configs:

| Example | Description |
|---------|-------------|
| `basic.json` | Volume + brightness + mic |
| `audio_panel.json` | Sliders + mute switches with `read_cmd` |
| `display_controls.json` | Brightness + night light + screen off |
| `quick_toggles.json` | Wi-Fi, Bluetooth, DND switches + buttons |
| `full_panel.json` | Combined sliders + switches + buttons |
| `complex_layout.json` | Two-column mixed layout |
| `nested_mixer.json` | Vertical sliders in nested boxes |
| `mpd_player.json` | MPD volume + transport + repeat/shuffle |
| `hyprland_tweaks.json` | Live-tweak gaps, borders, rounding |
| `gammarelay.json` | Color temperature + brightness + gamma |
| `per_app_volume.json` | Per-app PulseAudio volume |
| `themes/*.json` | Same config with each CSS theme |
