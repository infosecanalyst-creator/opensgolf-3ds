//! SimGolf native port: the platform independent core.
//!
//! Decoders for the original game's asset formats, its custom data formats, and the game rules as far as they are decoded
//! (see docs/PUBLISHER_EXE_NOTES.md). Nothing here opens a window or an audio device, so all of it can be unit tested.
//! No game data is part of this crate: everything is read at run time from the player's own copy of the game.

// The port keeps the original code's own constants (3.14159, 6.28) so its results stay identical; index loops mirror its tables.
#![allow(clippy::approx_constant, clippy::needless_range_loop, clippy::should_implement_trait)]

pub mod assets;
pub mod bink;
pub mod economy;
pub mod flc;
pub mod flight;
pub mod formats;
pub mod fsutil;
pub mod holes;
pub mod land;
pub mod mixer;
pub mod mood;
pub mod objects;
pub mod png;
pub mod properties;
pub mod rng;
pub mod shot;
pub mod sprites;
pub mod staff;
pub mod terrain;

pub use assets::{Indexed, Rgba};
