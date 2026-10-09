//! SimGolf native port: the platform independent core.
//!
//! Decoders for the original game's asset formats, its custom data formats, and the game rules as far as they are decoded
//! (see docs/PUBLISHER_EXE_NOTES.md). Nothing here opens a window or an audio device, so all of it can be unit tested.
//! No game data is part of this crate: everything is read at run time from the player's own copy of the game.

// The port keeps the original code's own constants (3.14159, 6.28) so its results stay identical; index loops mirror its tables.
#![allow(clippy::approx_constant, clippy::needless_range_loop, clippy::should_implement_trait)]

pub mod analysis;
pub mod assets;
pub mod bink;
pub mod bodies;
pub mod celebs;
pub mod championship;
pub mod clubhouse;
pub mod course;
pub mod decor;
pub mod economy;
pub mod flc;
pub mod flight;
pub mod formats;
pub mod fsutil;
pub mod geom;
pub mod golfer;
pub mod holes;
pub mod holetool;
pub mod homes;
pub mod land;
pub mod mixer;
pub mod mood;
pub mod objects;
pub mod planner;
pub mod png;
pub mod pro;
pub mod properties;
pub mod ratings;
pub mod records;
pub mod rng;
pub mod roster;
pub mod shot;
pub mod sounds;
pub mod sprites;
pub mod staff;
pub mod stories;
pub mod terrain;
pub mod thoughts;
pub mod top10;
pub mod tournament;
pub mod tracts;
pub mod vips;
pub mod wildlife;

pub use assets::{Indexed, Rgba};
