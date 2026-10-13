//! "Fix original bugs" (docs/FIXES.md): the rules of the simulation where the original game has a bug of its own. The
//! game turns the switch on from its setting (on by default); left off, as the tests and tools keep it, the simulation
//! follows the exe exactly. Per thread, as the game runs its simulation on the thread that reads its settings (and each
//! test keeps its own).
use std::cell::Cell;

thread_local! {
    static ON: Cell<bool> = const { Cell::new(false) };
}

/// Whether the original's bugs are fixed.
pub fn on() -> bool {
    ON.with(|c| c.get())
}

/// Sets the switch (the game does this from its setting).
pub fn set(on: bool) {
    ON.with(|c| c.set(on));
}
