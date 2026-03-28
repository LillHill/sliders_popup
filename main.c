/*
 * sliders_popup - Wayland popup with JSON-configured sliders
 *
 * Reads JSON config from stdin, creates a layer-shell popup with sliders.
 * Each slider runs a shell command with VAL set to the current value.
 * Styled via CSS (--css flag or "css" key in JSON).
 *
 * JSON format:
 * {
 *   "width": 300,
 *   "height": -1,
 *   "anchor": "right",
 *   "margin": { "top": 10, "right": 10, "bottom": 10, "left": 10 },
 *   "layer": "overlay",
 *   "namespace": "sliders_popup",
 *   "close_on_focus_loss": true,
 *   "css": "path/to/style.css",
 *   "orientation": "vertical",
 *   "sliders": [
 *     {
 *       "name": "volume",
 *       "label": "Volume",
 *       "cmd": "wpctl set-volume @DEFAULT_AUDIO_SINK@ ${VAL}%",
 *       "min": 0,
 *       "max": 100,
 *       "step": 1,
 *       "value": 50,
 *       "orientation": "horizontal"
 *     }
 *   ]
 * }
 */

#include <gtk/gtk.h>
#include <gtk-layer-shell/gtk-layer-shell.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>
#include "cJSON.h"

/* ── Slider state ── */
typedef struct {
    char *cmd;
    int  int_mode; /* 1 = format as int, 0 = format as double */
    double step;
} SliderData;

static void slider_data_free(gpointer data) {
    SliderData *sd = data;
    free(sd->cmd);
    free(sd);
}

static void on_slider_changed(GtkRange *range, gpointer user_data) {
    SliderData *sd = user_data;
    if (!sd->cmd || !sd->cmd[0]) return;

    double val = gtk_range_get_value(range);
    char val_str[64];
    if (sd->int_mode)
        snprintf(val_str, sizeof(val_str), "%d", (int)val);
    else
        snprintf(val_str, sizeof(val_str), "%.2f", val);

    /* Build env: VAL=value, then run cmd via sh */
    setenv("VAL", val_str, 1);
    /* Fire and forget with & so we don't block the UI */
    char *full_cmd = NULL;
    if (asprintf(&full_cmd, "%s &", sd->cmd) < 0) return;
    int ret = system(full_cmd);
    (void)ret;
    free(full_cmd);
}

/* ── Read all stdin ── */
static char *read_stdin(void) {
    size_t cap = 4096, len = 0;
    char *buf = malloc(cap);
    if (!buf) return NULL;
    ssize_t n;
    while ((n = read(STDIN_FILENO, buf + len, cap - len)) > 0) {
        len += n;
        if (len >= cap) { cap *= 2; buf = realloc(buf, cap); if (!buf) return NULL; }
    }
    buf[len] = '\0';
    return buf;
}

/* ── Helper to get string from JSON ── */
static const char *json_str(const cJSON *obj, const char *key, const char *def) {
    cJSON *v = cJSON_GetObjectItemCaseSensitive(obj, key);
    return (cJSON_IsString(v) && v->valuestring) ? v->valuestring : def;
}

static double json_num(const cJSON *obj, const char *key, double def) {
    cJSON *v = cJSON_GetObjectItemCaseSensitive(obj, key);
    return cJSON_IsNumber(v) ? v->valuedouble : def;
}

static int json_bool(const cJSON *obj, const char *key, int def) {
    cJSON *v = cJSON_GetObjectItemCaseSensitive(obj, key);
    if (!v) return def;
    if (cJSON_IsBool(v)) return cJSON_IsTrue(v);
    return def;
}

/* ── Set layer-shell anchors from string ── */
static void set_anchors(GtkWindow *win, const char *anchor) {
    if (!anchor) return;
    /* Reset all */
    gtk_layer_set_anchor(win, GTK_LAYER_SHELL_EDGE_TOP, FALSE);
    gtk_layer_set_anchor(win, GTK_LAYER_SHELL_EDGE_BOTTOM, FALSE);
    gtk_layer_set_anchor(win, GTK_LAYER_SHELL_EDGE_LEFT, FALSE);
    gtk_layer_set_anchor(win, GTK_LAYER_SHELL_EDGE_RIGHT, FALSE);

    /* Parse space/comma-separated anchors or single words */
    if (strstr(anchor, "top"))    gtk_layer_set_anchor(win, GTK_LAYER_SHELL_EDGE_TOP, TRUE);
    if (strstr(anchor, "bottom")) gtk_layer_set_anchor(win, GTK_LAYER_SHELL_EDGE_BOTTOM, TRUE);
    if (strstr(anchor, "left"))   gtk_layer_set_anchor(win, GTK_LAYER_SHELL_EDGE_LEFT, TRUE);
    if (strstr(anchor, "right"))  gtk_layer_set_anchor(win, GTK_LAYER_SHELL_EDGE_RIGHT, TRUE);
    if (strcmp(anchor, "center") == 0) { /* all false = centered */ }
}

static GtkLayerShellLayer parse_layer(const char *layer) {
    if (!layer) return GTK_LAYER_SHELL_LAYER_OVERLAY;
    if (strcmp(layer, "background") == 0) return GTK_LAYER_SHELL_LAYER_BACKGROUND;
    if (strcmp(layer, "bottom") == 0) return GTK_LAYER_SHELL_LAYER_BOTTOM;
    if (strcmp(layer, "top") == 0) return GTK_LAYER_SHELL_LAYER_TOP;
    return GTK_LAYER_SHELL_LAYER_OVERLAY;
}

/* Close on focus loss */
static gboolean on_focus_out(GtkWidget *widget, GdkEvent *event, gpointer data) {
    (void)event; (void)data;
    gtk_widget_destroy(widget);
    return TRUE;
}

static void on_window_destroy(GtkWidget *widget, gpointer data) {
    (void)widget; (void)data;
    gtk_main_quit();
}

/* ── Main ── */
int main(int argc, char **argv) {
    /* Check for --css flag before GTK eats args */
    const char *css_file_arg = NULL;
    for (int i = 1; i < argc; i++) {
        if (strcmp(argv[i], "--css") == 0 && i + 1 < argc) {
            css_file_arg = argv[++i];
        }
    }

    gtk_init(&argc, &argv);

    /* Read and parse JSON from stdin */
    char *input = read_stdin();
    if (!input || !input[0]) {
        fprintf(stderr, "sliders_popup: no JSON input on stdin\n");
        return 1;
    }

    cJSON *root = cJSON_Parse(input);
    free(input);
    if (!root) {
        fprintf(stderr, "sliders_popup: failed to parse JSON\n");
        return 1;
    }

    /* ── CSS ── */
    const char *css_path = css_file_arg ? css_file_arg : json_str(root, "css", NULL);
    if (css_path) {
        GtkCssProvider *css = gtk_css_provider_new();
        GError *err = NULL;
        if (css_path[0] == '/') {
            gtk_css_provider_load_from_path(css, css_path, &err);
        } else {
            /* Try relative to CWD */
            char abs[4096];
            if (getcwd(abs, sizeof(abs) - strlen(css_path) - 2)) {
                strcat(abs, "/");
                strcat(abs, css_path);
                gtk_css_provider_load_from_path(css, abs, &err);
            }
        }
        if (err) {
            fprintf(stderr, "sliders_popup: CSS error: %s\n", err->message);
            g_error_free(err);
        } else {
            gtk_style_context_add_provider_for_screen(
                gdk_screen_get_default(),
                GTK_STYLE_PROVIDER(css),
                GTK_STYLE_PROVIDER_PRIORITY_USER);
        }
        g_object_unref(css);
    }

    /* Also load inline CSS if "css_inline" is present */
    const char *css_inline = json_str(root, "css_inline", NULL);
    if (css_inline) {
        GtkCssProvider *css = gtk_css_provider_new();
        gtk_css_provider_load_from_data(css, css_inline, -1, NULL);
        gtk_style_context_add_provider_for_screen(
            gdk_screen_get_default(),
            GTK_STYLE_PROVIDER(css),
            GTK_STYLE_PROVIDER_PRIORITY_USER);
        g_object_unref(css);
    }

    /* ── Window setup ── */
    GtkWindow *win = GTK_WINDOW(gtk_window_new(GTK_WINDOW_TOPLEVEL));
    g_signal_connect(win, "destroy", G_CALLBACK(on_window_destroy), NULL);

    /* Set widget name for CSS targeting: window#sliders_popup */
    const char *ns = json_str(root, "namespace", "sliders_popup");
    gtk_widget_set_name(GTK_WIDGET(win), ns);

    /* Layer shell setup */
    gtk_layer_init_for_window(win);
    gtk_layer_set_layer(win, parse_layer(json_str(root, "layer", "overlay")));
    gtk_layer_set_namespace(win, ns);
    gtk_layer_set_keyboard_mode(win, GTK_LAYER_SHELL_KEYBOARD_MODE_ON_DEMAND);

    /* Anchors */
    set_anchors(win, json_str(root, "anchor", "right"));

    /* Margins */
    cJSON *margin = cJSON_GetObjectItemCaseSensitive(root, "margin");
    if (margin && cJSON_IsObject(margin)) {
        gtk_layer_set_margin(win, GTK_LAYER_SHELL_EDGE_TOP,    (int)json_num(margin, "top", 0));
        gtk_layer_set_margin(win, GTK_LAYER_SHELL_EDGE_BOTTOM, (int)json_num(margin, "bottom", 0));
        gtk_layer_set_margin(win, GTK_LAYER_SHELL_EDGE_LEFT,   (int)json_num(margin, "left", 0));
        gtk_layer_set_margin(win, GTK_LAYER_SHELL_EDGE_RIGHT,  (int)json_num(margin, "right", 0));
    } else if (margin && cJSON_IsNumber(margin)) {
        int m = margin->valueint;
        gtk_layer_set_margin(win, GTK_LAYER_SHELL_EDGE_TOP, m);
        gtk_layer_set_margin(win, GTK_LAYER_SHELL_EDGE_BOTTOM, m);
        gtk_layer_set_margin(win, GTK_LAYER_SHELL_EDGE_LEFT, m);
        gtk_layer_set_margin(win, GTK_LAYER_SHELL_EDGE_RIGHT, m);
    }

    /* Exclusive zone */
    int exclusive = (int)json_num(root, "exclusive_zone", -1);
    if (exclusive >= 0) gtk_layer_set_exclusive_zone(win, exclusive);

    /* Size */
    int width  = (int)json_num(root, "width", 300);
    int height = (int)json_num(root, "height", -1);
    if (width > 0 && height > 0)
        gtk_window_set_default_size(win, width, height);
    else if (width > 0)
        gtk_widget_set_size_request(GTK_WIDGET(win), width, -1);

    /* Close on focus loss */
    if (json_bool(root, "close_on_focus_loss", 1)) {
        g_signal_connect(win, "focus-out-event", G_CALLBACK(on_focus_out), NULL);
    }

    /* ── Container ── */
    const char *orient_str = json_str(root, "orientation", "vertical");
    GtkOrientation box_orient = (strcmp(orient_str, "horizontal") == 0)
        ? GTK_ORIENTATION_HORIZONTAL : GTK_ORIENTATION_VERTICAL;
    GtkWidget *box = gtk_box_new(box_orient, 4);
    gtk_widget_set_name(box, "sliders_box");
    gtk_container_set_border_width(GTK_CONTAINER(box), 8);
    gtk_container_add(GTK_CONTAINER(win), box);

    /* ── Sliders ── */
    cJSON *sliders = cJSON_GetObjectItemCaseSensitive(root, "sliders");
    if (!cJSON_IsArray(sliders)) {
        fprintf(stderr, "sliders_popup: 'sliders' array missing\n");
        cJSON_Delete(root);
        return 1;
    }

    int count = cJSON_GetArraySize(sliders);
    for (int i = 0; i < count; i++) {
        cJSON *s = cJSON_GetArrayItem(sliders, i);
        if (!cJSON_IsObject(s)) continue;

        const char *name  = json_str(s, "name", "slider");
        const char *label = json_str(s, "label", NULL);
        const char *cmd   = json_str(s, "cmd", "");
        double min_v  = json_num(s, "min", 0);
        double max_v  = json_num(s, "max", 100);
        double step   = json_num(s, "step", 1);
        double value  = json_num(s, "value", min_v);

        /* Per-slider orientation */
        const char *so = json_str(s, "orientation", NULL);
        GtkOrientation slider_orient = GTK_ORIENTATION_HORIZONTAL;
        if (so && strcmp(so, "vertical") == 0)
            slider_orient = GTK_ORIENTATION_VERTICAL;

        /* Container for label + slider */
        GtkWidget *row = gtk_box_new(
            (box_orient == GTK_ORIENTATION_VERTICAL) ? GTK_ORIENTATION_VERTICAL : GTK_ORIENTATION_HORIZONTAL, 2);
        gtk_widget_set_name(row, name);
        /* Add CSS class "slider-row" for styling */
        GtkStyleContext *row_ctx = gtk_widget_get_style_context(row);
        gtk_style_context_add_class(row_ctx, "slider-row");

        /* Label */
        if (label) {
            GtkWidget *lbl = gtk_label_new(label);
            gtk_widget_set_name(lbl, "slider_label");
            GtkStyleContext *lbl_ctx = gtk_widget_get_style_context(lbl);
            gtk_style_context_add_class(lbl_ctx, "slider-label");
            gtk_box_pack_start(GTK_BOX(row), lbl, FALSE, FALSE, 0);
        }

        /* Scale (slider) */
        GtkWidget *scale = gtk_scale_new_with_range(slider_orient, min_v, max_v, step);
        gtk_range_set_value(GTK_RANGE(scale), value);
        gtk_scale_set_draw_value(GTK_SCALE(scale), json_bool(s, "show_value", 1));
        gtk_widget_set_hexpand(scale, TRUE);
        gtk_widget_set_vexpand(scale, (slider_orient == GTK_ORIENTATION_VERTICAL));

        /* Value position */
        const char *vpos = json_str(s, "value_pos", "top");
        if (strcmp(vpos, "bottom") == 0)      gtk_scale_set_value_pos(GTK_SCALE(scale), GTK_POS_BOTTOM);
        else if (strcmp(vpos, "left") == 0)   gtk_scale_set_value_pos(GTK_SCALE(scale), GTK_POS_LEFT);
        else if (strcmp(vpos, "right") == 0)  gtk_scale_set_value_pos(GTK_SCALE(scale), GTK_POS_RIGHT);
        else                                  gtk_scale_set_value_pos(GTK_SCALE(scale), GTK_POS_TOP);

        /* Decimal digits: if step is integer, show 0 */
        int digits = (step == (int)step) ? 0 : 2;
        gtk_scale_set_digits(GTK_SCALE(scale), (int)json_num(s, "digits", digits));

        gtk_widget_set_name(scale, name);
        GtkStyleContext *sc_ctx = gtk_widget_get_style_context(scale);
        gtk_style_context_add_class(sc_ctx, "slider");

        /* Connect signal */
        SliderData *sd = malloc(sizeof(SliderData));
        sd->cmd = strdup(cmd);
        sd->int_mode = (step >= 1.0 && step == (int)step);
        sd->step = step;
        g_object_set_data_full(G_OBJECT(scale), "sd", sd, slider_data_free);
        g_signal_connect(scale, "value-changed", G_CALLBACK(on_slider_changed), sd);

        gtk_box_pack_start(GTK_BOX(row), scale, TRUE, TRUE, 0);
        gtk_box_pack_start(GTK_BOX(box), row, TRUE, TRUE, 0);
    }

    gtk_widget_show_all(GTK_WIDGET(win));
    gtk_main();

    cJSON_Delete(root);
    return 0;
}
