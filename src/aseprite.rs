use anyhow::{anyhow, Context};
use asefile::AsepriteFile;
use std::io::Cursor;

use crate::sprite::{crop_frames_to_union, frame_from_rgba_bytes, Frame};

const ASEPRITE_BYTES: &[u8] = include_bytes!("../assets/sprites/cat.aseprite");

pub struct AnimatedSprite {
    pub frames: Vec<Frame>,
    pub durations_ms: Vec<u32>,
}

pub fn load_tag(tag_name: &str) -> anyhow::Result<AnimatedSprite> {
    let ase = AsepriteFile::read(Cursor::new(ASEPRITE_BYTES))
        .context("parsing embedded aseprite file")?;

    eprintln!(
        "[scamp] aseprite: {} frames, {}x{} px",
        ase.num_frames(),
        ase.width(),
        ase.height()
    );

    let n_tags = ase.num_tags();
    eprintln!("[scamp] tags ({}):", n_tags);
    for i in 0..n_tags {
        let t = ase.tag(i);
        eprintln!(
            "[scamp]   '{}': frames {}..={}",
            t.name(),
            t.from_frame(),
            t.to_frame()
        );
    }

    let mut found: Option<asefile::Tag> = None;
    for i in 0..n_tags {
        let t = ase.tag(i);
        if t.name() == tag_name {
            found = Some(t.clone());
            break;
        }
    }
    let tag = found
        .ok_or_else(|| anyhow!("tag '{}' not found in aseprite file", tag_name))?;

    let mut frames = Vec::new();
    let mut durations = Vec::new();
    for idx in tag.from_frame()..=tag.to_frame() {
        let f = ase.frame(idx);
        let rgba = f.image();
        let (w, h) = (rgba.width(), rgba.height());
        frames.push(frame_from_rgba_bytes(rgba.as_raw(), w, h));
        durations.push(f.duration());
    }

    Ok(AnimatedSprite {
        frames: crop_frames_to_union(frames),
        durations_ms: durations,
    })
}
