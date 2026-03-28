{ config, lib, pkgs, ... }:

let
  cfg = config.programs.sliders_popup;
  pkg = cfg.package;

  # Convert a popup's settings attrset to a JSON file
  mkPopupJson = name: popup:
    pkgs.writeText "sliders_popup-${name}.json" (builtins.toJSON popup.settings);

  # Create a wrapper script for a popup
  mkPopupScript = name: popup:
    let
      jsonFile = mkPopupJson name popup;
      cssArgs = lib.optionalString (popup.css != null) "--css ${popup.css}";
    in
    pkgs.writeShellScriptBin "sliders_popup-${name}" ''
      exec ${pkg}/bin/sliders_popup ${cssArgs} < ${jsonFile}
    '';

  # All generated popup scripts as a single package
  allScripts = lib.mapAttrsToList mkPopupScript cfg.popups;

in
{
  options.programs.sliders_popup = {
    enable = lib.mkEnableOption "sliders_popup Wayland popup widgets";

    package = lib.mkOption {
      type = lib.types.package;
      description = "The sliders_popup package to use.";
    };

    popups = lib.mkOption {
      type = lib.types.attrsOf (lib.types.submodule ({ name, ... }: {
        options = {
          settings = lib.mkOption {
            type = lib.types.attrs;
            default = {};
            description = ''
              The JSON configuration as a Nix attribute set.
              Converted to JSON and piped to sliders_popup on launch.
            '';
            example = lib.literalExpression ''
              {
                width = 350;
                anchor = "top right";
                margin = { top = 10; right = 10; };
                layer = "overlay";
                close_on_focus_loss = true;
                orientation = "vertical";
                children = [
                  {
                    type = "slider";
                    name = "volume";
                    label = "Volume";
                    cmd = "wpctl set-volume @DEFAULT_AUDIO_SINK@ \''${VAL}%";
                    min = 0; max = 100; step = 1;
                    read_cmd = "wpctl get-volume @DEFAULT_AUDIO_SINK@ | awk '{printf \"%.0f\", $2 * 100}'";
                  }
                ];
              }
            '';
          };

          css = lib.mkOption {
            type = lib.types.nullOr lib.types.path;
            default = null;
            description = "Optional CSS file path (overrides css in settings).";
          };

          command = lib.mkOption {
            type = lib.types.str;
            readOnly = true;
            description = "The full command to launch this popup. Use this in keybinds.";
          };

          script = lib.mkOption {
            type = lib.types.package;
            readOnly = true;
            description = "The generated wrapper script package.";
          };
        };

        config = {
          script = mkPopupScript name config;
          command = "${mkPopupScript name config}/bin/sliders_popup-${name}";
        };
      }));
      default = {};
      description = "Named popup configurations.";
    };
  };

  config = lib.mkIf cfg.enable {
    home.packages = [ pkg ] ++ allScripts;
  };
}
