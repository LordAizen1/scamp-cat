# scamp

![scamp running in Windows Terminal: a small white cat sitting on an empty PowerShell prompt](assets/screenshot.png)

A small animated cat that lives in your terminal and keeps you company while you work.

She wanders, sits, washes her paws, yawns, scratches, and sleeps in different poses. Walks left, right, up, and down with proper four-direction animations. Ignores you when she runs full-screen TUIs like vim or htop. Cleans up after herself when shell output scrolls past her.

**Three different cats ship with scamp** (gray, ginger tabby, white) and one is picked at random every time you launch. You can pin one by setting the `SCAMP_CAT` env var (see [Configure](#configure) below).

## Important: run scamp in Windows Terminal

scamp is built around **sixel graphics** for sharp pixel-art rendering. Sixel works in:

- **Windows Terminal** (recommended for Windows users; install free from the Microsoft Store, or `winget install Microsoft.WindowsTerminal`)
- WezTerm, iTerm2, foot, kitty, contour, recent xterm

In regular PowerShell windows, cmd.exe, and most IDE-integrated terminals (VS Code, Cursor, Antigravity), you get a **half-block fallback** that works but looks chunkier and less detailed. The cat is still a cat, just lower fidelity.

If you can, open Windows Terminal first, then launch scamp inside it. The visual difference is significant.

## Install

Pre-built binaries for every major platform are on the [latest release](https://github.com/LordAizen1/scamp-cat/releases/latest) page. No Rust toolchain needed.

### Windows

Download `scamp-x86_64-windows.exe`, double-click. Opens in Windows Terminal automatically if installed, otherwise in ConHost with the half-block fallback.

### Linux

```
wget https://github.com/LordAizen1/scamp-cat/releases/latest/download/scamp-x86_64-linux
chmod +x scamp-x86_64-linux
./scamp-x86_64-linux
```

### macOS

```
# Apple Silicon (M1, M2, M3, M4)
curl -LO https://github.com/LordAizen1/scamp-cat/releases/latest/download/scamp-aarch64-macos
chmod +x scamp-aarch64-macos
./scamp-aarch64-macos

# Intel Mac
curl -LO https://github.com/LordAizen1/scamp-cat/releases/latest/download/scamp-x86_64-macos
chmod +x scamp-x86_64-macos
./scamp-x86_64-macos
```

### From source (any platform with Rust)

```
cargo install --git https://github.com/LordAizen1/scamp-cat
scamp
```

## Configure

Two environment variables.

`SCAMP_CAT` pins a specific cat color (default behavior is **random per launch** so each session feels a little different):
```
$env:SCAMP_CAT="gray"     # the dark-gray cat
$env:SCAMP_CAT="ginger"   # the orange tabby
$env:SCAMP_CAT="white"    # the white cat
scamp
```
Unset to go back to the random pick on each launch.

`SCAMP_RENDERER` overrides the auto-detected renderer:
```
$env:SCAMP_RENDERER="sixel"      # force pixel-perfect sixel
$env:SCAMP_RENDERER="halfblock"  # force the half-block fallback
```

By default scamp picks sixel when it detects a sixel-capable terminal (Windows Terminal, WezTerm, iTerm2, Konsole, ghostty, foot, mlterm, contour) and half-block everywhere else.

## What's inside

- PTY wrapper (`portable-pty`) hosts your shell and forwards bytes both ways
- ANSI parser (`vte`) maintains a screen-cell model used to clean up after the sprite
- Sixel encoder (`icy_sixel`) for pixel-perfect rendering
- Half-block character renderer for fallback compatibility
- Atomic compositor: single buffered write per redraw, scroll-aware ghost cleanup, alt-screen pause, terminal resize handling

## Terminal compatibility

scamp looks her best in terminals that support sixel graphics. Terminals that don't get the half-block fallback (still cute, just chunkier).

### Sharp pixel-art (sixel, auto-detected)

| Platform | Terminal                                 |
|----------|------------------------------------------|
| Windows  | Windows Terminal                         |
| macOS    | iTerm2, WezTerm, Ghostty                 |
| Linux    | Konsole (KDE default), WezTerm, Ghostty, foot, mlterm, contour |

### Chunky half-block fallback

| Platform | Terminal                                                             |
|----------|----------------------------------------------------------------------|
| Windows  | ConHost (default cmd / PowerShell windows), VS Code / Cursor / Antigravity terminals |
| macOS    | Terminal.app                                                         |
| Linux    | gnome-terminal (Ubuntu / Fedora default), Alacritty, kitty, xfce4-terminal, terminator, urxvt, tilix |

A note for Linux users: most popular default terminals (gnome-terminal especially) don't support sixel, so the chunky fallback is what you'll see out of the box on a fresh Ubuntu / Fedora install. To get the sharp version, install one of the supported terminals above (Konsole, WezTerm, Ghostty, etc.) and run scamp inside it.

A note for kitty users: kitty has its own graphics protocol that's separate from sixel. scamp doesn't support kitty's protocol yet, so kitty users get the half-block fallback for now. Adding it is on the roadmap.

### Forcing a renderer

If you know your terminal supports sixel but auto-detect missed it, force it:
```
SCAMP_RENDERER=sixel scamp
```
Or force half-block if sixel renders weirdly:
```
SCAMP_RENDERER=halfblock scamp
```

For VS Code / Cursor / Antigravity terminal specifically: enable `terminal.integrated.enableImages` in your settings, then run with `SCAMP_RENDERER=sixel`.

## Credits

Cat sprites by **Last tick** ([animated-pixel-kittens-cats-32x32](https://last-tick.itch.io/animated-pixel-kittens-cats-32x32) on itch.io). Used under the asset pack's license: free for personal and commercial use, redistribution as standalone art prohibited. The sprites are bundled into the scamp binary as part of a software product.

If you like the art, support the artist by visiting their itch.io page.

## License

scamp's source code is MIT licensed. See [LICENSE](LICENSE).

The bundled sprite assets are subject to Last tick's terms (see Credits above).
