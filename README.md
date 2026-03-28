# sliders_popup

Lightweight Wayland popup with JSON-configured sliders. Written in C with GTK3 + gtk-layer-shell.

## Build

```
# Dependencies (Debian/Ubuntu)
sudo apt install libgtk-3-dev libgtk-layer-shell-dev

# Build
make

# Install (optional)
sudo make install
```

## Usage

Pipe JSON config to stdin:

```bash
sliders_popup < config.json
# or
cat config.json | sliders_popup
# or with CSS override
echo '{"sliders":[...]}' | sliders_popup --css /path/to/style.css
```

## JSON Config

```json
{
  "width": 300,
  "height": -1,
  "anchor": "right",
  "margin": { "top": 10, "right": 10, "bottom": 10, "left": 10 },
  "layer": "overlay",
  "namespace": "sliders_popup",
  "close_on_focus_loss": true,
  "css": "style.css",
  "css_inline": "scale slider { background: red; }",
  "orientation": "vertical",
  "exclusive_zone": -1,
  "sliders": [
    {
      "name": "volume",
      "label": "Volume",
      "cmd": "wpctl set-volume @DEFAULT_AUDIO_SINK@ ${VAL}%",
      "min": 0,
      "max": 100,
      "step": 1,
      "value": 50,
      "orientation": "horizontal",
      "show_value": true,
      "value_pos": "top",
      "digits": 0
    }
  ]
}
```

### Top-level options

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `width` | int | 300 | Window width in pixels |
| `height` | int | -1 | Window height (-1 = auto) |
| `anchor` | string | `"right"` | Edge(s): `"top"`, `"bottom"`, `"left"`, `"right"`, `"top right"`, `"center"` |
| `margin` | object/int | 0 | Edge margins (object with top/bottom/left/right, or single int) |
| `layer` | string | `"overlay"` | Layer: `"background"`, `"bottom"`, `"top"`, `"overlay"` |
| `namespace` | string | `"sliders_popup"` | Layer shell namespace / CSS widget name |
| `close_on_focus_loss` | bool | true | Close popup when it loses focus |
| `css` | string | null | Path to CSS file |
| `css_inline` | string | null | Inline CSS string |
| `orientation` | string | `"vertical"` | Layout direction: `"vertical"` or `"horizontal"` |
| `exclusive_zone` | int | -1 | Exclusive zone for layer shell |

### Slider options

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `name` | string | `"slider"` | Widget name (for CSS: `#name`) |
| `label` | string | null | Optional label text |
| `cmd` | string | `""` | Shell command — `$VAL` is set to the slider value |
| `min` | number | 0 | Minimum value |
| `max` | number | 100 | Maximum value |
| `step` | number | 1 | Step increment |
| `value` | number | min | Initial value |
| `orientation` | string | `"horizontal"` | Slider direction |
| `show_value` | bool | true | Show numeric value on slider |
| `value_pos` | string | `"top"` | Value position: `"top"`, `"bottom"`, `"left"`, `"right"` |
| `digits` | int | auto | Decimal places to display |

## CSS Styling

Style with standard GTK3 CSS. The popup sets these widget names and classes:

- `#sliders_popup` (or your `namespace`) — the window
- `#sliders_box` — the container box
- `.slider-row` / `#<name>` — each slider's row
- `.slider-label` — label widgets
- `.slider` / `#<name>` — the scale widgets

See `style.css` for a Catppuccin-themed example.

## How `cmd` works

When a slider value changes, the command is executed via `sh -c` with the environment variable `VAL` set to the current value. Integer steps produce integer values (e.g. `50`), fractional steps produce decimals (e.g. `0.75`).

```json
"cmd": "wpctl set-volume @DEFAULT_AUDIO_SINK@ ${VAL}%"
```
