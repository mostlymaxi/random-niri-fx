# home-manager module: `programs.random-fx`.
self:
{
  config,
  lib,
  pkgs,
  ...
}:

let
  inherit (lib)
    literalExpression
    mkDefault
    mkEnableOption
    mkIf
    mkMerge
    mkOption
    types
    ;

  cfg = config.programs.random-fx;

  # base16 slots for each time of day: blue night, violet pre-dawn, orange
  # dawn, yellow morning, cyan noon, orange afternoon, red sunset, violet dusk.
  stylixColors = with config.lib.stylix.colors.withHashtag; {
    palette = {
      "00:00" = base0D;
      "05:00" = base0E;
      "07:00" = base09;
      "10:00" = base0A;
      "13:00" = base0C;
      "17:00" = base09;
      "19:00" = base08;
      "21:30" = base0E;
    };
    hot = base08;
    zap = base0C;
  };

  render =
    v:
    if builtins.isList v then
      lib.concatMapStringsSep ", " toString v
    else if builtins.isAttrs v then
      # Palette: `"07:00" = "#ff8a7a"` pairs; zero-padded times sort correctly.
      lib.concatStringsSep ", " (lib.mapAttrsToList (time: color: "${time} ${color}") v)
    else
      toString v;

  settings =
    cfg.settings
    // lib.optionalAttrs (cfg.palette != { }) { inherit (cfg) palette; }
    // lib.optionalAttrs (cfg.hot != null) { inherit (cfg) hot; }
    // lib.optionalAttrs (cfg.zap != null) { inherit (cfg) zap; };

  color = types.strMatching "#[0-9a-fA-F]{6}";
in
{
  options.programs.random-fx = {
    enable = mkEnableOption "random-fx, a thin line along the edges of the screen";

    package = mkOption {
      type = types.package;
      default = self.packages.${pkgs.stdenv.hostPlatform.system}.default;
      defaultText = literalExpression "random-fx.packages.\${system}.default";
      description = "The random-fx package to use.";
    };

    settings = mkOption {
      type =
        with types;
        attrsOf (oneOf [
          str
          int
          float
          (listOf str)
        ]);
      default = { };
      example = literalExpression ''
        {
          dwell = "3-5";
          flight = 2.2;
          rest = 1.4;
          states = [ "sweep" "heat" "zap" "chime" ];
          clock = "2291 1395 261 41";
        }
      '';
      description = "Entries for `~/.config/random-fx/config`. See the README for keys.";
    };

    palette = mkOption {
      type = types.attrsOf color;
      default = { };
      example = {
        "06:00" = "#ff8a7a";
        "13:00" = "#9be7ff";
        "20:00" = "#af7ad6";
      };
      description = ''
        Line color by local time of day (`HH:MM`), blended in between and
        wrapping around midnight. Empty uses the built-in palette.
      '';
    };

    hot = mkOption {
      type = types.nullOr color;
      default = null;
      description = "Color the line runs toward under heavy CPU load.";
    };

    zap = mkOption {
      type = types.nullOr color;
      default = null;
      description = "Default color for `random-fx zap`.";
    };

    stylix.enable = mkOption {
      type = types.bool;
      default = config.stylix.enable or false;
      defaultText = literalExpression "config.stylix.enable";
      description = "Take `palette`, `hot` and `zap` from the Stylix base16 scheme.";
    };

    systemd = {
      enable = mkOption {
        type = types.bool;
        default = true;
        description = "Run random-fx as a systemd user service.";
      };
      target = mkOption {
        type = types.str;
        default = "graphical-session.target";
        description = "The systemd target that starts and stops random-fx.";
      };
    };
  };

  config = mkIf cfg.enable (mkMerge [
    (mkIf cfg.stylix.enable {
      programs.random-fx = {
        # Per entry, so overriding one time keeps the rest of the scheme.
        palette = lib.mapAttrs (_: mkDefault) stylixColors.palette;
        hot = mkDefault stylixColors.hot;
        zap = mkDefault stylixColors.zap;
      };
    })
    {
      home.packages = [ cfg.package ];

      xdg.configFile."random-fx/config".text = lib.concatStrings (
        lib.mapAttrsToList (key: value: "${key} = ${render value}\n") settings
      );

      systemd.user.services.random-fx = mkIf cfg.systemd.enable {
        Unit = {
          Description = "random-fx screen-edge line";
          PartOf = [ cfg.systemd.target ];
          After = [ cfg.systemd.target ];
          X-Restart-Triggers = [ "${config.xdg.configFile."random-fx/config".source}" ];
        };
        Service = {
          ExecStart = lib.getExe cfg.package;
          Restart = "on-failure";
          RestartSec = 2;
        };
        Install.WantedBy = [ cfg.systemd.target ];
      };
    }
  ]);
}
