# random-fx

A line that lives on the edge of the screen. It sweeps across the top once
per cycle, and its mood changes with what the machine is doing.

| state   | when                          | looks like                                                   |
|---------|-------------------------------|--------------------------------------------------------------|
| `sweep` | nothing else is going on      | a comet in the time-of-day color                             |
| `heat`  | CPU above ~60%                | red, shimmering, sweeping faster                             |
| `zap`   | `random-fx zap [#rrggbb]`, or now and then on its own | two streaks race around the edges and collide bottom-right |
| `chime` | on the hour and half hour, or `random-fx chime` | the same race, then the energy wraps around the waybar clock pill and lights it up |

The line moves like kitty's cursor trail: the head flies across on a bezier
curve and the tail chases it with exponential decay. A state machine picks
the most-wanted state between sweeps, once the current state has played for
its `dwell` (a random 3–5s by default). Events (`zap`) skip the dwell but
still wait for the next sweep, so the rhythm never breaks.

## Install (home-manager)

```nix
# flake.nix
inputs.random-fx.url = "github:…/random-fx";  # or path:/home/you/build/random-fx
inputs.random-fx.inputs.nixpkgs.follows = "nixpkgs";

# home config
imports = [ inputs.random-fx.homeModules.default ];
programs.random-fx.enable = true;
```

This runs a systemd user service on `graphical-session.target`. With Stylix
enabled, the colors come from your base16 scheme automatically:

| time  | slot     | time  | slot     |
|-------|----------|-------|----------|
| 00:00 | `base0D` | 13:00 | `base0C` |
| 05:00 | `base0E` | 17:00 | `base09` |
| 07:00 | `base09` | 19:00 | `base08` |
| 10:00 | `base0A` | 21:30 | `base0E` |

`hot` is `base08` and `zap` is `base0C`. Any of them can be overridden:

```nix
programs.random-fx = {
  palette."13:00" = "#ffffff";
  zap = config.lib.stylix.colors.withHashtag.base0B;
  settings = {
    dwell = "3-5";
    # The waybar clock pill's border box, `x y w h` in screen pixels.
    clock = "2291 1395 261 41";
  };
};
```

Zap on every commit, as a git hook (`.git/hooks/post-commit`):

    #!/bin/sh
    random-fx zap >/dev/null 2>&1 || true

## Config

`~/.config/random-fx/config` (written by the module, or by hand). All keys
are optional; `#` followed by a space starts a comment.

    dwell = 3-5      # seconds per state, random in range (or a single number)
    flight = 2.2     # seconds to cross the screen
    rest = 1.4       # seconds between sweeps
    height = 24      # strip depth in px
    states = sweep, heat, zap, chime
    palette = 00:00 #6a5acd, 07:00 #ff8a7a, 13:00 #9be7ff, 19:00 #f2609e
    hot = #ff3b30
    zap = #bfe9ff
    zap_chance = 0.01  # random zap (in the line's color) each time a state is picked
    clock = 2291 1395 261 41  # waybar clock pill for `chime`: x y w h, border box

Without `clock`, the chime still runs its race but has nothing to wrap.

The palette blends between the given times and wraps around midnight.

## Development

    nix develop
    cargo run --release           # daemon
    cargo run --release -- zap    # send an event

### Adding a state

Write `src/states/<name>.rs` implementing `State` (`want` = how much it wants
to play given the current `Signals`, `update` = style the line for this
frame), then add it to `states::all()`. Motion is handled for you
(`src/motion.rs`); states only style the line. For a bigger moment, a state
can also put lines on any screen edge or around the clock pill with
`overlay` (see `zap` and `chime`). New data
sources go in `src/signals/`.
