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
    sliders: Vec<SliderConfig>,
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
struct SliderConfig {
    #[serde(default = "default_slider_name")]
    name: String,
    #[serde(default)]
    label: Option<String>,
    #[serde(default)]
    cmd: String,
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
}

fn default_width() -> i32 { 300 }
fn default_neg() -> i32 { -1 }
fn default_anchor() -> String { "right".into() }
fn default_layer() -> String { "overlay".into() }
fn default_namespace() -> String { "sliders_popup".into() }
fn default_true() -> bool { true }
fn default_vertical() -> String { "vertical".into() }
fn default_slider_name() -> String { "slider".into() }
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

fn run_cmd(cmd: &str, val: &str) {
    let _ = Command::new("sh")
        .arg("-c")
        .arg(cmd)
        .env("VAL", val)
        .spawn();
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

    // Layer shell — trait methods from LayerShell
    win.init_layer_shell();
    win.set_layer(parse_layer(&config.layer));
    win.set_namespace(&config.namespace);
    win.set_keyboard_mode(gtk_layer_shell::KeyboardMode::OnDemand);

    // Anchors — reset all, then enable requested
    use gtk_layer_shell::Edge;
    for edge in &[Edge::Top, Edge::Bottom, Edge::Left, Edge::Right] {
        win.set_anchor(*edge, false);
    }
    for edge in parse_edges(&config.anchor) {
        win.set_anchor(edge, true);
    }

    // Margins
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

    // Exclusive zone
    if config.exclusive_zone >= 0 {
        win.set_exclusive_zone(config.exclusive_zone);
    }

    // Size
    if config.width > 0 && config.height > 0 {
        win.set_default_size(config.width, config.height);
    } else if config.width > 0 {
        win.set_size_request(config.width, -1);
    }

    // Close on focus loss
    if config.close_on_focus_loss {
        win.connect_focus_out_event(|w, _| {
            unsafe { w.destroy(); }
            gtk::glib::Propagation::Stop
        });
    }

    win.connect_destroy(|_| gtk::main_quit());

    // ── Container ──
    let orient = if config.orientation == "horizontal" {
        gtk::Orientation::Horizontal
    } else {
        gtk::Orientation::Vertical
    };
    let container = gtk::Box::new(orient, 4);
    container.set_widget_name("sliders_box");
    container.set_border_width(8);
    win.add(&container);

    // ── Sliders ──
    for sc in &config.sliders {
        let row_orient = if orient == gtk::Orientation::Vertical {
            gtk::Orientation::Vertical
        } else {
            gtk::Orientation::Horizontal
        };
        let row = gtk::Box::new(row_orient, 2);
        row.set_widget_name(&sc.name);
        row.style_context().add_class("slider-row");

        // Label
        if let Some(ref label_text) = sc.label {
            let label = gtk::Label::new(Some(label_text));
            label.set_widget_name("slider_label");
            label.style_context().add_class("slider-label");
            row.pack_start(&label, false, false, 0);
        }

        // Scale
        let slider_orient = match sc.orientation.as_deref() {
            Some("vertical") => gtk::Orientation::Vertical,
            _ => gtk::Orientation::Horizontal,
        };
        let initial = sc.value.unwrap_or(sc.min);
        let scale = gtk::Scale::with_range(slider_orient, sc.min, sc.max, sc.step);
        scale.set_value(initial);
        scale.set_draw_value(sc.show_value);
        if slider_orient == gtk::Orientation::Vertical {
            let len = sc.length.unwrap_or(200);
            scale.set_size_request(-1, len);
            scale.set_vexpand(true);
        } else {
            let len = sc.length.unwrap_or(-1);
            if len > 0 {
                scale.set_size_request(len, -1);
            }
            scale.set_hexpand(true);
        }

        // Value position
        scale.set_value_pos(match sc.value_pos.as_str() {
            "bottom" => gtk::PositionType::Bottom,
            "left" => gtk::PositionType::Left,
            "right" => gtk::PositionType::Right,
            _ => gtk::PositionType::Top,
        });

        // Digits
        let digits = sc.digits.unwrap_or_else(|| {
            if sc.step == sc.step.floor() && sc.step >= 1.0 { 0 } else { 2 }
        });
        scale.set_digits(digits);

        scale.set_widget_name(&sc.name);
        scale.style_context().add_class("slider");

        // Command callback
        let cmd = sc.cmd.clone();
        let int_mode = sc.step >= 1.0 && sc.step == sc.step.floor();
        scale.connect_value_changed(move |s| {
            let val = s.value();
            let val_str = if int_mode {
                format!("{}", val as i64)
            } else {
                format!("{:.2}", val)
            };
            if !cmd.is_empty() {
                run_cmd(&cmd, &val_str);
            }
        });

        row.pack_start(&scale, true, true, 0);
        container.pack_start(&row, true, true, 0);
    }

    win.show_all();
    gtk::main();
}
