# scamp

A small animated cat that lives in your terminal and keeps you company while you work.

She wanders, sits, washes her paws, yawns, scratches, and sleeps in different poses. Walks left, right, up, and down with proper four-direction animations. Ignores you when she runs full-screen TUIs like vim or htop. Cleans up after herself when shell output scrolls past her.

## Install

### Windows (no Rust needed)

Download `scamp.exe` from the [latest release](https://github.com/LordAizen1/scamp-cat/releases/latest), then in any terminal:

```
scamp.exe
```

Best in **Windows Terminal** (sixel renderer, sharp pixel-art cat). Works in regular PowerShell, cmd, and IDE-integrated terminals as well, with a chunkier half-block fallback.

### From source (any platform with Rust)

```
cargo install --git https://github.com/LordAizen1/scamp-cat
scamp
```

## Configure

Two environment variables.

`SCAMP_CAT` picks a color (default: random per launch):
```
$env:SCAMP_CAT="ginger"   # gray | ginger | white
scamp
```

`SCAMP_RENDERER` overrides the auto-detected renderer:
```
$env:SCAMP_RENDERER="sixel"      # force pixel-perfect sixel
$env:SCAMP_RENDERER="halfblock"  # force the half-block fallback
```

By default scamp picks sixel when it detects a sixel-capable terminal (Windows Terminal, WezTerm, iTerm2, foot, kitty, contour) and half-block everywhere else.

## What's inside

- PTY wrapper (`portable-pty`) hosts your shell and forwards bytes both ways
- ANSI parser (`vte`) maintains a screen-cell model used to clean up after the sprite
- Sixel encoder (`icy_sixel`) for pixel-perfect rendering
- Half-block character renderer for fallback compatibility
- Atomic compositor: single buffered write per redraw, scroll-aware ghost cleanup, alt-screen pause, terminal resize handling

## Compat notes

| Terminal                    | Renderer    | Notes                              |
|-----------------------------|-------------|------------------------------------|
| Windows Terminal            | sixel       | First-class, looks great           |
| WezTerm, iTerm2, foot, kitty| sixel       | Detected via env vars              |
| ConHost (default cmd / PS)  | half-block  | Fallback, smaller and chunkier     |
| VS Code / Cursor / similar  | half-block  | Sixel possible if you enable it    |

To get the cat to look its best in your IDE terminal, enable `terminal.integrated.enableImages` in settings, then `$env:SCAMP_RENDERER="sixel"` before launching.

## Credits

Cat sprites by **Last tick** ([animated-pixel-kittens-cats-32x32](https://last-tick.itch.io/animated-pixel-kittens-cats-32x32) on itch.io). Used under the asset pack's license: free for personal and commercial use, redistribution as standalone art prohibited. The sprites are bundled into the scamp binary as part of a software product.

If you like the art, support the artist by visiting their itch.io page.

## License

scamp's source code is MIT licensed. See [LICENSE](LICENSE).

The bundled sprite assets are subject to Last tick's terms (see Credits above).
