//! Layout system for the dashboard.
//!
//! Provides predefined layout presets that determine which widgets are displayed
//! and how they are arranged. Users switch between layouts using keyboard
//! shortcuts (`1` for default, `2` for compact).

mod presets;

pub use presets::{Layout, LayoutManager};
