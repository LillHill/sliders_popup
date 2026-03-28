use gtk::prelude::*;
use gtk::CssProvider;
use gtk_layer_shell::LayerShell;
use serde::Deserialize;
use std::io::Read;
use std::process::Command;

// ── JSON schema ──

#[derive(Deserialize, Debug)]
struct Config {
    #[serde(default = "default_width")]
    width: i32,
    #[serde(default = "default_neg")]
    height: i32,
    #[serde(default = "default_anchor")]
    anchor: String,
    #[serde(default)]
    margin: Margin,
    #[serde(default = "default_layer")]
    layer: String,
    #[serde(default = "default_namespace")]
    namespace: String,
    #[serde(default = "default_true")]
    close_on_focus_loss: bool,
    #[serde(default)]
    css: Option<String>,
    #[serde(default)]
    css_inline: Option<String>,
    #[serde(default = "default_vertical")]
    orientation: String,
    #[serde(default = "default_neg")]
    exclusive_zone: i32,
    #[serde(default)]
    spacing: Option<i32>,
    // New tree-based layout
    #[serde(default)]
    children: Vec<Widget>,
    // Backward compat: flat slider list
    #[serde(default)]
    sliders: Vec<Widget>,
}

#[derive(Deserialize, Debug, Default)]
#[serde(untagged)]
enum Margin {
    #[default]
    None,
    Uniform(i32),
    Sides {
        #[serde(default)]
        top: i32,
        #[serde(default)]
        bottom: i32,
        #[serde(default)]
        left: i32,
        #[serde(default)]
        right: i32,
    },
}

#[derive(Deserialize, Debug)]
#[serde(tag = "type")]
#[serde(rename_all = "lowercase")]
enum Widget {
    Box {
        #[serde(default = "default_horizontal")]
        orientation: String,
        #[serde(default)]
        name: Option<String>,
        #[serde(default)]
        spacing: Option<i32>,
        #[serde(default)]
        homogeneous: bool,
        #[serde(default)]
        children: Vec<Widget>,
    },
    Slider {
        #[serde(default = "default_widget_name")]
        name: String,
        #[serde(default)]
        label: Option<String>,
        #[serde(default)]
        cmd: String,
        #[serde(default)]
        read_cmd: Option<String>,
        #[serde(default)]
        min: f64,
        #[serde(default = "default_max")]
        max: f64,
        #[serde(default = "default_step")]
        step: f64,
        #[serde(default)]
        value: Option<f64>,
        #[serde(default)]
        orientation: Option<String>,
        #[serde(default = "default_true")]
        show_value: bool,
        #[serde(default = "default_value_pos")]
        value_pos: String,
        #[serde(default)]
        digits: Option<i32>,
        #[serde(default)]
        length: Option<i32>,
    },
    Button {
        #[serde(default = "default_widget_name")]
        name: String,
        #[serde(default)]
        label: Option<String>,
        #[serde(default)]
        cmd: String,
    },
    Switch {
        #[serde(default = "default_widget_name")]
        name: String,
        #[serde(default)]
        label: Option<String>,
        #[serde(default)]
        cmd: String,
        #[serde(default)]
        read_cmd: Option<String>,
        #[serde(default)]
        value: bool,
    },
    // Untagged fallback: if no "type" field, treat as slider (backward compat)
    #[serde(untagged)]
    SliderCompat {
        #[serde(default = "default_widget_name")]
        name: String,
        #[serde(default)]
        label: Option<String>,
        #[serde(default)]
        cmd: String,
        #[serde(default)]
        read_cmd: Option<String>,
        #[serde(default)]
        min: f64,
        #[serde(default = "default_max")]
        max: f64,
        #[serde(default = "default_step")]
        step: f64,
        #[serde(default)]
        value: Option<f64>,
        #[serde(default)]
        orientation: Option<String>,
        #[serde(default = "default_true")]
        show_value: bool,
        #[serde(default = "default_value_pos")]
        value_pos: String,
        #[serde(default)]
        digits: Option<i32>,
        #[serde(default)]
        length: Option<i32>,
    },
}

fn default_width() -> i32 { 300 }
fn default_neg() -> i32 { -1 }
fn default_anchor() -> String { "right".into() }
fn default_layer() -> String { "overlay".into() }
fn default_namespace() -> String { "sliders_popup".into() }
fn default_true() -> bool { true }
fn default_vertical() -> String { "vertical".into() }
fn default_horizontal() -> String { "horizontal".into() }
fn default_widget_name() -> String { "widget".into() }
fn default_max() -> f64 { 100.0 }
fn default_step() -> f64 { 1.0 }
fn default_value_pos() -> String { "top".into() }

// ── Helpers ──

fn parse_layer(s: &str) -> gtk_layer_shell::Layer {
    match s {
        "background" => gtk_layer_shell::Layer::Background,
        "bottom" => gtk_layer_shell::Layer::Bottom,
        "top" => gtk_layer_shell::Layer::Top,
        _ => gtk_layer_shell::Layer::Overlay,
    }
}

fn parse_edges(s: &str) -> Vec<gtk_layer_shell::Edge> {
    let mut edges = Vec::new();
    if s.contains("top") { edges.push(gtk_layer_shell::Edge::Top); }
    if s.contains("bottom") { edges.push(gtk_layer_shell::Edge::Bottom); }
    if s.contains("left") { edges.push(gtk_layer_shell::Edge::Left); }
    if s.contains("right") { edges.push(gtk_layer_shell::Edge::Right); }
    edges
}

fn parse_orient(s: &str) -> gtk::Orientation {
    if s == "horizontal" { gtk::Orientation::Horizontal }
    else { gtk::Orientation::Vertical }
}

fn run_cmd(cmd: &str, val: &str) {
    let _ = Command::new("sh")
        .arg("-c")
        .arg(cmd)
        .env("VAL", val)
        .spawn();
}

fn read_value(cmd: &str) -> Option<String> {
    Command::new("sh")
        .arg("-c")
        .arg(cmd)
        .output()
        .ok()
        .and_then(|o| {
            if o.status.success() {
                let s = String::from_utf8_lossy(&o.stdout).trim().to_string();
                if s.is_empty() { None } else { Some(s) }
            } else {
                None
            }
        })
}

// ── Widget builder ──

fn build_widget(widget: &Widget, parent: &gtk::Box) {
    match widget {
        Widget::Box { orientation, name, spacing, homogeneous, children } => {
            let bx = gtk::Box::new(parse_orient(orientation), spacing.unwrap_or(4));
            bx.set_homogeneous(*homogeneous);
            if let Some(n) = name {
                bx.set_widget_name(n);
            }
            bx.style_context().add_class("container");
            for child in children {
                build_widget(child, &bx);
            }
            parent.pack_start(&bx, true, true, 0);
        }

        Widget::Slider { name, label, cmd, read_cmd, min, max, step, value,
                         orientation, show_value, value_pos, digits, length }
        | Widget::SliderCompat { name, label, cmd, read_cmd, min, max, step, value,
                                 orientation, show_value, value_pos, digits, length } => {
            let row = gtk::Box::new(gtk::Orientation::Vertical, 2);
            row.set_widget_name(name);
            row.style_context().add_class("slider-row");

            if let Some(ref text) = label {
                let lbl = gtk::Label::new(Some(text));
                lbl.style_context().add_class("slider-label");
                row.pack_start(&lbl, false, false, 0);
            }

            let slider_orient = match orientation.as_deref() {
                Some("vertical") => gtk::Orientation::Vertical,
                _ => gtk::Orientation::Horizontal,
            };

            // Resolve initial value: read_cmd > value > min
            let initial = read_cmd.as_deref()
                .and_then(|rc| read_value(rc))
                .and_then(|s| s.parse::<f64>().ok())
                .or(*value)
                .unwrap_or(*min);

            let scale = gtk::Scale::with_range(slider_orient, *min, *max, *step);
            scale.set_value(initial);
            scale.set_draw_value(*show_value);

            if slider_orient == gtk::Orientation::Vertical {
                let len = length.unwrap_or(200);
                scale.set_size_request(-1, len);
                scale.set_vexpand(true);
            } else {
                let len = length.unwrap_or(-1);
                if len > 0 { scale.set_size_request(len, -1); }
                scale.set_hexpand(true);
            }

            scale.set_value_pos(match value_pos.as_str() {
                "bottom" => gtk::PositionType::Bottom,
                "left" => gtk::PositionType::Left,
                "right" => gtk::PositionType::Right,
                _ => gtk::PositionType::Top,
            });

            let d = digits.unwrap_or_else(|| {
                if *step == step.floor() && *step >= 1.0 { 0 } else { 2 }
            });
            scale.set_digits(d);
            scale.set_widget_name(name);
            scale.style_context().add_class("slider");

            let cmd = cmd.clone();
            let int_mode = *step >= 1.0 && *step == step.floor();
            scale.connect_value_changed(move |s| {
                let val = s.value();
                let val_str = if int_mode {
                    format!("{}", val as i64)
                } else {
                    format!("{:.2}", val)
                };
                if !cmd.is_empty() { run_cmd(&cmd, &val_str); }
            });

            row.pack_start(&scale, true, true, 0);
            parent.pack_start(&row, true, true, 0);
        }

        Widget::Button { name, label, cmd } => {
            let text = label.as_deref().unwrap_or(name);
            let btn = gtk::Button::with_label(text);
            btn.set_widget_name(name);
            btn.style_context().add_class("popup-button");
            let cmd = cmd.clone();
            btn.connect_clicked(move |_| {
                if !cmd.is_empty() { run_cmd(&cmd, ""); }
            });
            parent.pack_start(&btn, false, false, 0);
        }

        Widget::Switch { name, label, cmd, read_cmd, value } => {
            let row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
            row.set_widget_name(name);
            row.style_context().add_class("switch-row");

            if let Some(ref text) = label {
                let lbl = gtk::Label::new(Some(text));
                lbl.style_context().add_class("switch-label");
                lbl.set_hexpand(true);
                lbl.set_xalign(0.0);
                row.pack_start(&lbl, true, true, 0);
            }

            // Resolve initial state: read_cmd > value
            let initial = read_cmd.as_deref()
                .and_then(|rc| read_value(rc))
                .map(|s| matches!(s.trim(), "1" | "true" | "yes" | "on"))
                .unwrap_or(*value);

            let switch = gtk::Switch::new();
            switch.set_active(initial);
            switch.set_widget_name(name);
            switch.style_context().add_class("popup-switch");

            let cmd = cmd.clone();
            switch.connect_state_set(move |_, state| {
                let val_str = if state { "1" } else { "0" };
                if !cmd.is_empty() { run_cmd(&cmd, val_str); }
                gtk::glib::Propagation::Proceed
            });

            row.pack_end(&switch, false, false, 0);
            parent.pack_start(&row, false, false, 0);
        }
    }
}

fn main() {
    // ── Read stdin ──
    let mut input = String::new();
    let args: Vec<String> = std::env::args().collect();
    let css_file_arg = args.windows(2)
        .find(|w| w[0] == "--css")
        .map(|w| w[1].clone());

    if let Err(e) = std::io::stdin().read_to_string(&mut input) {
        eprintln!("sliders_popup: failed to read stdin: {e}");
        std::process::exit(1);
    }

    if input.trim().is_empty() {
        eprintln!("sliders_popup: no JSON input on stdin");
        eprintln!("  hint: pipe JSON config, e.g.: sliders_popup < config.json");
        std::process::exit(1);
    }

    // ── Parse JSON with detailed errors ──
    let config: Config = match serde_json::from_str(&input) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("sliders_popup: failed to parse JSON: {e}");
            eprintln!("  line {}, column {}", e.line(), e.column());
            let lines: Vec<&str> = input.lines().collect();
            let err_line = e.line().saturating_sub(1);
            let start = err_line.saturating_sub(2);
            let end = (err_line + 3).min(lines.len());
            eprintln!("  context:");
            for i in start..end {
                let marker = if i == err_line { " >> " } else { "    " };
                eprintln!("{marker}{:>3} | {}", i + 1, lines.get(i).unwrap_or(&""));
            }
            std::process::exit(1);
        }
    };

    // ── GTK init ──
    gtk::init().expect("sliders_popup: failed to init GTK");

    // ── CSS ──
    let css_path = css_file_arg.as_deref().or(config.css.as_deref());
    if let Some(path) = css_path {
        let provider = CssProvider::new();
        let abs = if path.starts_with('/') {
            path.to_string()
        } else {
            let cwd = std::env::current_dir().unwrap_or_default();
            format!("{}/{}", cwd.display(), path)
        };
        match provider.load_from_path(&abs) {
            Ok(()) => {
                gtk::StyleContext::add_provider_for_screen(
                    &gtk::gdk::Screen::default().expect("no screen"),
                    &provider,
                    gtk::STYLE_PROVIDER_PRIORITY_USER,
                );
            }
            Err(e) => eprintln!("sliders_popup: CSS load error ({abs}): {e}"),
        }
    }

    if let Some(ref inline) = config.css_inline {
        let provider = CssProvider::new();
        provider.load_from_data(inline.as_bytes()).unwrap_or_else(|e| {
            eprintln!("sliders_popup: inline CSS error: {e}");
        });
        gtk::StyleContext::add_provider_for_screen(
            &gtk::gdk::Screen::default().expect("no screen"),
            &provider,
            gtk::STYLE_PROVIDER_PRIORITY_USER,
        );
    }

    // ── Window ──
    let win = gtk::Window::new(gtk::WindowType::Toplevel);
    win.set_widget_name(&config.namespace);

    win.init_layer_shell();
    win.set_layer(parse_layer(&config.layer));
    win.set_namespace(&config.namespace);
    win.set_keyboard_mode(gtk_layer_shell::KeyboardMode::OnDemand);

    use gtk_layer_shell::Edge;
    for edge in &[Edge::Top, Edge::Bottom, Edge::Left, Edge::Right] {
        win.set_anchor(*edge, false);
    }
    for edge in parse_edges(&config.anchor) {
        win.set_anchor(edge, true);
    }

    match &config.margin {
        Margin::None => {}
        Margin::Uniform(m) => {
            for edge in &[Edge::Top, Edge::Bottom, Edge::Left, Edge::Right] {
                win.set_layer_shell_margin(*edge, *m);
            }
        }
        Margin::Sides { top, bottom, left, right } => {
            win.set_layer_shell_margin(Edge::Top, *top);
            win.set_layer_shell_margin(Edge::Bottom, *bottom);
            win.set_layer_shell_margin(Edge::Left, *left);
            win.set_layer_shell_margin(Edge::Right, *right);
        }
    }

    if config.exclusive_zone >= 0 {
        win.set_exclusive_zone(config.exclusive_zone);
    }

    if config.width > 0 && config.height > 0 {
        win.set_default_size(config.width, config.height);
    } else if config.width > 0 {
        win.set_size_request(config.width, -1);
    }

    if config.close_on_focus_loss {
        win.connect_focus_out_event(|w, _| {
            unsafe { w.destroy(); }
            gtk::glib::Propagation::Stop
        });
    }

    win.connect_destroy(|_| gtk::main_quit());

    // ── Container ──
    let container = gtk::Box::new(parse_orient(&config.orientation), config.spacing.unwrap_or(4));
    container.set_widget_name("sliders_box");
    container.set_border_width(8);
    win.add(&container);

    // ── Build widgets ──
    // Prefer "children" if present, fall back to "sliders" for compat
    let widgets = if !config.children.is_empty() {
        &config.children
    } else {
        &config.sliders
    };

    for w in widgets {
        build_widget(w, &container);
    }

    win.show_all();
    gtk::main();
}
