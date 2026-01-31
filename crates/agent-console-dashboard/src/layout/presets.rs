//! Built-in layout preset definitions and layout manager.
//!
//! Two built-in layouts are provided for v0:
//! - **default** (shortcut `1`): Two-line session status with API usage
//! - **compact** (shortcut `2`): One-line session status with API usage
//!
//! Custom layout configuration is deferred to v2+.

/// A layout preset defining which widgets to display.
#[derive(Debug, Clone)]
pub struct Layout {
    /// Human-readable name of the layout.
    pub name: String,
    /// Ordered list of widget identifiers to render.
    pub widget_ids: Vec<String>,
}

impl Layout {
    /// Creates a new layout with the given name and widget IDs.
    pub fn new(name: &str, widget_ids: &[&str]) -> Self {
        Self {
            name: name.to_string(),
            widget_ids: widget_ids.iter().map(|s| s.to_string()).collect(),
        }
    }
}

/// Manages layout presets and tracks the active layout.
#[derive(Debug, Clone)]
pub struct LayoutManager {
    /// Available layout presets indexed by name.
    layouts: Vec<Layout>,
    /// Index of the currently active layout.
    active_index: usize,
}

impl LayoutManager {
    /// Creates a new LayoutManager with built-in presets.
    ///
    /// The default layout is active initially.
    pub fn new() -> Self {
        let layouts = vec![
            Layout::new("default", &["session-status", "api-usage"]),
            Layout::new("compact", &["session-status:compact", "api-usage"]),
        ];
        Self {
            layouts,
            active_index: 0,
        }
    }

    /// Switches to the layout at the given 1-based index.
    ///
    /// Returns `true` if the switch succeeded, `false` if the index is invalid.
    pub fn switch_by_index(&mut self, index: u8) -> bool {
        let zero_based = (index as usize).checked_sub(1);
        match zero_based {
            Some(i) if i < self.layouts.len() => {
                self.active_index = i;
                true
            }
            _ => false,
        }
    }

    /// Returns the currently active layout.
    pub fn active_layout(&self) -> &Layout {
        &self.layouts[self.active_index]
    }

    /// Returns the 1-based index of the active layout.
    pub fn active_index(&self) -> u8 {
        (self.active_index + 1) as u8
    }

    /// Returns the count of available layouts.
    pub fn layout_count(&self) -> usize {
        self.layouts.len()
    }

    /// Returns an iterator over all layout names.
    pub fn layout_names(&self) -> impl Iterator<Item = &str> {
        self.layouts.iter().map(|l| l.name.as_str())
    }
}

impl Default for LayoutManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_layout_new() {
        let layout = Layout::new("test", &["widget-a", "widget-b"]);
        assert_eq!(layout.name, "test");
        assert_eq!(layout.widget_ids, vec!["widget-a", "widget-b"]);
    }

    #[test]
    fn test_layout_manager_default_active() {
        let manager = LayoutManager::new();
        assert_eq!(manager.active_layout().name, "default");
        assert_eq!(manager.active_index(), 1);
    }

    #[test]
    fn test_layout_manager_switch_to_compact() {
        let mut manager = LayoutManager::new();
        assert!(manager.switch_by_index(2));
        assert_eq!(manager.active_layout().name, "compact");
        assert_eq!(manager.active_index(), 2);
    }

    #[test]
    fn test_layout_manager_switch_back_to_default() {
        let mut manager = LayoutManager::new();
        manager.switch_by_index(2);
        assert!(manager.switch_by_index(1));
        assert_eq!(manager.active_layout().name, "default");
    }

    #[test]
    fn test_layout_manager_invalid_index_returns_false() {
        let mut manager = LayoutManager::new();
        assert!(!manager.switch_by_index(0));
        assert!(!manager.switch_by_index(3));
        assert!(!manager.switch_by_index(255));
        // Active should remain default
        assert_eq!(manager.active_layout().name, "default");
    }

    #[test]
    fn test_layout_manager_layout_count() {
        let manager = LayoutManager::new();
        assert_eq!(manager.layout_count(), 2);
    }

    #[test]
    fn test_layout_manager_layout_names() {
        let manager = LayoutManager::new();
        let names: Vec<&str> = manager.layout_names().collect();
        assert_eq!(names, vec!["default", "compact"]);
    }

    #[test]
    fn test_default_layout_widgets() {
        let manager = LayoutManager::new();
        let layout = manager.active_layout();
        assert_eq!(layout.widget_ids, vec!["session-status", "api-usage"]);
    }

    #[test]
    fn test_compact_layout_widgets() {
        let mut manager = LayoutManager::new();
        manager.switch_by_index(2);
        let layout = manager.active_layout();
        assert_eq!(
            layout.widget_ids,
            vec!["session-status:compact", "api-usage"]
        );
    }

    #[test]
    fn test_layout_manager_default_trait() {
        let manager = LayoutManager::default();
        assert_eq!(manager.active_layout().name, "default");
    }

    #[test]
    fn test_layout_clone() {
        let layout = Layout::new("test", &["a", "b"]);
        let cloned = layout.clone();
        assert_eq!(cloned.name, "test");
        assert_eq!(cloned.widget_ids, vec!["a", "b"]);
    }

    #[test]
    fn test_layout_manager_clone() {
        let mut manager = LayoutManager::new();
        manager.switch_by_index(2);
        let cloned = manager.clone();
        assert_eq!(cloned.active_layout().name, "compact");
    }

    #[test]
    fn test_layout_debug() {
        let layout = Layout::new("debug-test", &["w1"]);
        let debug = format!("{:?}", layout);
        assert!(debug.contains("debug-test"));
    }
}
