//! The xorshift32 generator used throughout the port (demo course, golfer variety, shot spread). It is our own choice,
//! not the original's generator, which is not decoded.

#[derive(Clone, Copy, Debug)]
pub struct Rng {
    pub s: u32,
}

impl Rng {
    pub fn new(seed: u32) -> Self {
        Rng { s: seed }
    }
    pub fn next(&mut self) -> u32 {
        self.s ^= self.s << 13;
        self.s ^= self.s >> 17;
        self.s ^= self.s << 5;
        self.s
    }
    /// 0..1 with 24 bits of precision.
    pub fn unit(&mut self) -> f32 {
        (self.next() & 0xFF_FFFF) as f32 / 0x100_0000 as f32
    }
    pub fn range(&mut self, n: i32) -> i32 {
        (self.next() % n as u32) as i32
    }
}
