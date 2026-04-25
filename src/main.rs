mod aseprite;
mod pet;
mod screen;
mod sixel;
mod sprite;

use crossterm::terminal::{disable_raw_mode, enable_raw_mode, size};
use portable_pty::{native_pty_system, CommandBuilder, PtySize};
use std::fmt::Write as FmtWrite;
use std::io::{Read, Stdout, Write};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use pet::{Anim, Pet};
use screen::{dump_to_string, Screen};

const DEBUG_DUMP_KEY: u8 = 0x1c;
const TICK_MS: u64 = 100;

const SHEET_GRAY: &[u8] = include_bytes!("../assets/sprites/cat_gray.png");
const SHEET_GINGER: &[u8] = include_bytes!("../assets/sprites/cat_ginger.png");
const SHEET_WHITE: &[u8] = include_bytes!("../assets/sprites/cat_white.png");

fn pick_sheet() -> (&'static [u8], &'static str) {
    use rand::Rng;
    let choice = std::env::var("SCAMP_CAT").ok();
    match choice.as_deref() {
        Some("gray") | Some("grey") => (SHEET_GRAY, "gray"),
        Some("ginger") | Some("orange") | Some("tabby") => (SHEET_GINGER, "ginger"),
        Some("white") => (SHEET_WHITE, "white"),
        _ => {
            let sheets = [(SHEET_GRAY, "gray"), (SHEET_GINGER, "ginger"), (SHEET_WHITE, "white")];
            sheets[rand::thread_rng().gen_range(0..sheets.len())]
        }
    }
}

const SOURCE_FRAME_PX: u32 = 32; // each cell in sprite sheet
const TARGET_FRAME_PX: u32 = 64; // upscaled for sixel emit (Nearest, preserves pixels)

// Approximate Windows Terminal cell pixel size, used to compute how many
// terminal cells the sprite occupies. Slight overestimate is safer (we may
// blank one extra cell on erase, which is fine).
const CELL_PIXEL_W: u32 = 9;
const CELL_PIXEL_H: u32 = 18;

const IDLE_FRAMES: &[(u32, u32)] = &[
    (0, 1152), (32, 1152), (64, 1152), (96, 1152),
    (128, 1152), (160, 1152), (192, 1152), (224, 1152),
];
const WALK_RIGHT_FRAMES: &[(u32, u32)] = &[
    (0, 192), (32, 192), (64, 192), (96, 192),
    (128, 192), (160, 192), (192, 192), (224, 192),
];
const WALK_LEFT_FRAMES: &[(u32, u32)] = &[
    (0, 224), (32, 224), (64, 224), (96, 224),
    (128, 224), (160, 224), (192, 224), (224, 224),
];
const WALK_DOWN_FRAMES: &[(u32, u32)] = &[(0, 128), (32, 128), (64, 128), (96, 128)];
const WALK_UP_FRAMES: &[(u32, u32)] = &[(0, 160), (32, 160), (64, 160), (96, 160)];
// Sleep poses (single static frames).
const SLEEP_CURL_FRAMES: &[(u32, u32)] = &[(0, 512)];
const SLEEP_LOAF_FRAMES: &[(u32, u32)] = &[(0, 384)];
const SLEEP_HEAD_FRAMES: &[(u32, u32)] = &[(0, 448)];
const SLEEP_STRETCH_FRAMES: &[(u32, u32)] = &[(0, 576)];
// Idle variations.
const YAWN_FRAMES: &[(u32, u32)] = &[
    (0, 1024), (32, 1024), (64, 1024), (96, 1024),
    (128, 1024), (160, 1024), (192, 1024), (224, 1024),
];
const WASH_LIE_FRAMES: &[(u32, u32)] = &[
    (0, 1216), (32, 1216), (64, 1216), (96, 1216),
    (128, 1216), (160, 1216), (192, 1216),
];
const SCRATCH_FRAMES: &[(u32, u32)] = &[
    (0, 1280), (32, 1280), (64, 1280), (96, 1280),
    (128, 1280), (160, 1280), (192, 1280), (224, 1280),
];
const IDLE_FRAME_MS: u32 = 250;
const WALK_FRAME_MS: u32 = 100;
const SLEEP_FRAME_MS: u32 = 1000;

fn pick_shell() -> String {
    if let Ok(s) = std::env::var("SHELL") {
        return s;
    }
    if cfg!(windows) {
        for candidate in ["pwsh.exe", "powershell.exe", "cmd.exe"] {
            if which::which(candidate).is_ok() {
                return candidate.into();
            }
        }
        std::env::var("COMSPEC").unwrap_or_else(|_| "cmd.exe".into())
    } else {
        "/bin/sh".into()
    }
}

fn dump_screen(screen: &Screen) -> std::io::Result<std::path::PathBuf> {
    let dir = dirs::home_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join(".scamp");
    std::fs::create_dir_all(&dir)?;
    let path = dir.join("screen.txt");
    std::fs::write(&path, dump_to_string(screen))?;
    Ok(path)
}

fn build_animations() -> anyhow::Result<(Vec<Anim>, u16, u16)> {
    let (sheet_bytes, sheet_name) = pick_sheet();
    eprintln!("[scamp] cat: {}", sheet_name);
    let img = image::load_from_memory(sheet_bytes)?;
    let mut anims = Vec::new();
    // Animation index ordering (must match pet.rs ANIM_* consts):
    //   0 = WASH SIT       (idle)
    //   1 = WALK_RIGHT
    //   2 = WALK_LEFT
    //   3 = WALK_UP
    //   4 = WALK_DOWN
    //   5 = SLEEP_CURL
    //   6 = YAWN           (idle variation)
    //   7 = SLEEP_LOAF
    //   8 = SLEEP_HEAD
    //   9 = SLEEP_STRETCH
    //  10 = WASH_LIE       (idle variation)
    //  11 = SCRATCH        (idle variation)
    for (coords, dur_ms) in [
        (IDLE_FRAMES, IDLE_FRAME_MS),
        (WALK_RIGHT_FRAMES, WALK_FRAME_MS),
        (WALK_LEFT_FRAMES, WALK_FRAME_MS),
        (WALK_UP_FRAMES, WALK_FRAME_MS),
        (WALK_DOWN_FRAMES, WALK_FRAME_MS),
        (SLEEP_CURL_FRAMES, SLEEP_FRAME_MS),
        (YAWN_FRAMES, IDLE_FRAME_MS),
        (SLEEP_LOAF_FRAMES, SLEEP_FRAME_MS),
        (SLEEP_HEAD_FRAMES, SLEEP_FRAME_MS),
        (SLEEP_STRETCH_FRAMES, SLEEP_FRAME_MS),
        (WASH_LIE_FRAMES, IDLE_FRAME_MS),
        (SCRATCH_FRAMES, IDLE_FRAME_MS),
    ] {
        let mut sixels = Vec::with_capacity(coords.len());
        for &(x, y) in coords {
            let region = img.crop_imm(x, y, SOURCE_FRAME_PX, SOURCE_FRAME_PX);
            // Nearest-neighbor upscale preserves crisp pixel art.
            let scaled = region.resize_exact(
                TARGET_FRAME_PX,
                TARGET_FRAME_PX,
                image::imageops::FilterType::Nearest,
            );
            let rgba = scaled.to_rgba8();
            let (w, h) = (rgba.width(), rgba.height());
            sixels.push(sixel::encode_rgba(rgba.as_raw(), w, h)?);
        }
        let durations_ms = vec![dur_ms; sixels.len()];
        anims.push(Anim { sixels, durations_ms });
    }

    let cell_w = ((TARGET_FRAME_PX + CELL_PIXEL_W - 1) / CELL_PIXEL_W) as u16;
    let cell_h = ((TARGET_FRAME_PX + CELL_PIXEL_H - 1) / CELL_PIXEL_H) as u16;
    Ok((anims, cell_w, cell_h))
}

fn render_pet(pet: &mut Pet, screen: &Screen, stdout: &mut Stdout) {
    let now = (pet.row, pet.col, pet.anim_index(), pet.current_frame);
    let needs_redraw = pet.last_drawn != Some(now);
    if !needs_redraw {
        return;
    }

    let mut buf = String::with_capacity(8192);
    buf.push_str("\x1b7\x1b[?25l"); // save cursor + hide

    // Restore old footprint from screen model so shell text isn't smudged.
    if let Some((or, oc, _, _)) = pet.last_drawn {
        for cy in 0..pet.cell_h {
            for cx in 0..pet.cell_w {
                let r = or + cy;
                let c = oc + cx;
                if r >= screen.rows || c >= screen.cols {
                    continue;
                }
                let ch = screen.cell_at(r, c).ch;
                let _ = write!(
                    buf,
                    "\x1b[{};{}H\x1b[0m{}",
                    r + 1,
                    c + 1,
                    if ch == '\0' { ' ' } else { ch }
                );
            }
        }
    }

    // Position cursor at top-left of pet cell-area, emit sixel.
    let _ = write!(buf, "\x1b[{};{}H", pet.row + 1, pet.col + 1);
    buf.push_str(pet.current_sixel());

    buf.push_str("\x1b[0m\x1b8\x1b[?25h");
    let _ = stdout.write_all(buf.as_bytes());
    let _ = stdout.flush();
    pet.last_drawn = Some(now);
}

fn main() -> anyhow::Result<()> {
    let (cols, rows) = size().unwrap_or((100, 30));

    let (animations, cell_w, cell_h) = build_animations()?;

    let pty_system = native_pty_system();
    let pair = pty_system.openpty(PtySize {
        rows,
        cols,
        pixel_width: 0,
        pixel_height: 0,
    })?;

    let shell = pick_shell();
    let cmd = CommandBuilder::new(shell);
    let mut child = pair.slave.spawn_command(cmd)?;
    drop(pair.slave);

    let mut reader = pair.master.try_clone_reader()?;
    let mut writer = pair.master.take_writer()?;

    let screen = Arc::new(Mutex::new(Screen::new(cols, rows)));
    let pet = Arc::new(Mutex::new(Pet::new(rows, cols, animations, cell_w, cell_h)));
    let stdout_lock = Arc::new(Mutex::new(std::io::stdout()));

    enable_raw_mode()?;

    let pet_r = Arc::clone(&pet);
    let screen_r = Arc::clone(&screen);
    let stdout_r = Arc::clone(&stdout_lock);
    thread::spawn(move || {
        let mut parser = vte::Parser::new();
        let mut rbuf = [0u8; 4096];
        loop {
            match reader.read(&mut rbuf) {
                Ok(0) | Err(_) => break,
                Ok(n) => {
                    // Lock order matches the tick thread: pet → screen → stdout.
                    let mut p = pet_r.lock().unwrap();
                    let mut s = screen_r.lock().unwrap();
                    let mut out = stdout_r.lock().unwrap();
                    for &b in &rbuf[..n] {
                        parser.advance(&mut *s, b);
                    }
                    if out.write_all(&rbuf[..n]).is_err() {
                        break;
                    }
                    p.last_drawn = None;
                    render_pet(&mut p, &s, &mut out);
                }
            }
        }
    });

    let screen_w = Arc::clone(&screen);
    thread::spawn(move || {
        let mut buf = [0u8; 4096];
        let mut stdin = std::io::stdin();
        loop {
            match stdin.read(&mut buf) {
                Ok(0) | Err(_) => break,
                Ok(n) => {
                    let mut send_start = 0;
                    for i in 0..n {
                        if buf[i] == DEBUG_DUMP_KEY {
                            if send_start < i
                                && writer.write_all(&buf[send_start..i]).is_err()
                            {
                                return;
                            }
                            let s = screen_w.lock().unwrap();
                            let _ = dump_screen(&s);
                            send_start = i + 1;
                        }
                    }
                    if send_start < n && writer.write_all(&buf[send_start..n]).is_err() {
                        break;
                    }
                    let _ = writer.flush();
                }
            }
        }
    });

    let pet_t = Arc::clone(&pet);
    let screen_t = Arc::clone(&screen);
    let stdout_t = Arc::clone(&stdout_lock);
    thread::spawn(move || loop {
        thread::sleep(Duration::from_millis(TICK_MS));
        let mut p = pet_t.lock().unwrap();
        let s = screen_t.lock().unwrap();
        let mut out = stdout_t.lock().unwrap();
        p.tick();
        render_pet(&mut p, &s, &mut out);
    });

    let _ = child.wait();
    disable_raw_mode()?;

    let mut stdout = stdout_lock.lock().unwrap();
    let _ = write!(stdout, "\x1b[?25h\r\n");
    let _ = stdout.flush();
    Ok(())
}
