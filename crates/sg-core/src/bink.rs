//! Bink 1 file header (the game's two videos are BIKi files). Decoding is left to ffmpeg, see sgplay.

#[derive(Clone, Debug, Default)]
pub struct BinkHeader {
    /// "BIK" + revision letter.
    pub magic: [u8; 4],
    pub file_size: u32,
    pub frames: u32,
    pub largest_frame: u32,
    pub width: u32,
    pub height: u32,
    pub fps_num: u32,
    pub fps_den: u32,
    pub flags: u32,
    pub audio_tracks: u32,
}

impl BinkHeader {
    pub fn fps(&self) -> f64 {
        self.fps_num as f64 / self.fps_den.max(1) as f64
    }
}

pub fn parse_bink_header(d: &[u8]) -> Option<BinkHeader> {
    if d.len() < 44 || &d[0..3] != b"BIK" {
        return None;
    }
    let u = |o: usize| u32::from_le_bytes([d[o], d[o + 1], d[o + 2], d[o + 3]]);
    let h = BinkHeader {
        magic: [d[0], d[1], d[2], d[3]],
        file_size: u(4).wrapping_add(8),
        frames: u(8),
        largest_frame: u(12),
        width: u(20),
        height: u(24),
        fps_num: u(28),
        fps_den: if u(32) != 0 { u(32) } else { 1 },
        flags: u(36),
        audio_tracks: u(40),
    };
    (h.width > 0 && h.height > 0 && h.width <= 4096 && h.height <= 4096 && h.fps_num > 0).then_some(h)
}
