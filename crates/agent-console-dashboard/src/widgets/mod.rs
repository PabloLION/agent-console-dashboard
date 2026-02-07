//! Widget system for the agent console dashboard.
//!
//! This module defines the `Widget` trait that all dashboard widgets must
//! implement, and the `WidgetRegistry` for dynamic widget creation.
//!
//! # Architecture
//!
//! Widgets are small, composable UI components that render a single line
//! of content in the dashboard status bar. Each widget receives a
//! `WidgetContext` containing shared application state and renders
//! itself into a ratatui `Line`.
//!
//! The `WidgetRegistry` maps widget identifiers to factory functions,
//! allowing dynamic widget creation from configuration or user input.
//!
//! # Example
//!
//! ```
//! use agent_console::widgets::{Widget, WidgetContext, WidgetRegistry};
//! use agent_console::Session;
//! use ratatui::text::Line;
//!
//! let registry = WidgetRegistry::new();
//! let ids = registry.available_ids();
//! assert!(!ids.is_empty());
//!
//! let widget = registry.create("clock").expect("clock widget exists");
//! let sessions: Vec<Session> = vec![];
//! let ctx = WidgetContext::new(&sessions);
//! let line = widget.render(40, &ctx);
//! ```

pub mod api_usage;
pub mod context;
pub mod session_status;
pub mod working_dir;

pub use context::WidgetContext;

use ratatui::text::Line;
use std::collections::HashMap;

/// Trait for dashboard widgets.
///
/// Each widget renders a single [`Line`] of content given a width
/// constraint and shared context. Widgets must be thread-safe
/// (`Send + Sync`) to support concurrent rendering pipelines.
///
/// # Required Methods
///
/// - [`render`](Widget::render): Produce a line of styled text.
/// - [`id`](Widget::id): Return a unique static identifier.
/// - [`min_width`](Widget::min_width): Minimum columns needed for meaningful output.
pub trait Widget: Send + Sync {
    /// Render the widget content as a single line.
    ///
    /// # Arguments
    ///
    /// * `width` - Available horizontal space in columns.
    /// * `context` - Shared application state for rendering.
    fn render(&self, width: u16, context: &WidgetContext) -> Line<'_>;

    /// Unique identifier for this widget type.
    fn id(&self) -> &'static str;

    /// Minimum width in columns required for useful output.
    ///
    /// If the available width is less than this value, the widget
    /// may choose to render a truncated or empty representation.
    fn min_width(&self) -> u16;
}

/// Factory function type for creating widget instances.
pub type WidgetFactory = fn() -> Box<dyn Widget>;

/// Registry mapping widget identifiers to factory functions.
///
/// The registry is pre-populated with placeholder factories for all
/// known widget IDs. As real widget implementations are added, their
/// factories replace the placeholders.
///
/// # Example
///
/// ```
/// use agent_console::widgets::WidgetRegistry;
///
/// let registry = WidgetRegistry::new();
/// assert!(registry.create("clock").is_some());
/// assert!(registry.create("nonexistent").is_none());
/// ```
pub struct WidgetRegistry {
    factories: HashMap<&'static str, WidgetFactory>,
}

impl WidgetRegistry {
    /// Creates a new registry with built-in placeholder widgets.
    ///
    /// The following widget IDs are registered by default:
    /// - `session-status`
    /// - `working-dir`
    /// - `api-usage`
    /// - `state-history`
    /// - `clock`
    /// - `spacer`
    pub fn new() -> Self {
        let mut reg = Self {
            factories: HashMap::new(),
        };
        let builtin_ids: &[&str] = &[
            "session-status",
            "working-dir",
            "api-usage",
            "state-history",
            "clock",
            "spacer",
        ];
        for &id in builtin_ids {
            reg.factories.insert(id, placeholder_factory(id));
        }
        reg
    }

    /// Register a widget factory for the given identifier.
    ///
    /// Overwrites any existing factory for the same ID.
    pub fn register(&mut self, id: &'static str, factory: WidgetFactory) {
        self.factories.insert(id, factory);
    }

    /// Create a widget instance by identifier.
    ///
    /// Returns `None` if no factory is registered for the given ID.
    pub fn create(&self, id: &str) -> Option<Box<dyn Widget>> {
        self.factories.get(id).map(|f| f())
    }

    /// List all registered widget identifiers.
    ///
    /// The order is not guaranteed.
    pub fn available_ids(&self) -> Vec<&'static str> {
        self.factories.keys().copied().collect()
    }
}

impl Default for WidgetRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Placeholder widget used until real implementations exist
// ---------------------------------------------------------------------------

/// A placeholder widget that renders its ID as plain text.
///
/// Used as a stand-in for widget types that have not yet been
/// implemented. Each placeholder displays `[<id>]` when rendered.
struct PlaceholderWidget {
    widget_id: &'static str,
}

impl Widget for PlaceholderWidget {
    fn render(&self, _width: u16, _context: &WidgetContext) -> Line<'_> {
        Line::raw(format!("[{}]", self.widget_id))
    }

    fn id(&self) -> &'static str {
        self.widget_id
    }

    fn min_width(&self) -> u16 {
        // ID length + brackets + 1 padding
        (self.widget_id.len() as u16) + 3
    }
}

/// Returns a factory function that creates a [`PlaceholderWidget`] for
/// the given ID.
///
/// This uses a macro-style approach: each known ID gets its own factory
/// closure that captures the static string at compile time.
fn placeholder_factory(id: &'static str) -> WidgetFactory {
    // We need a concrete fn pointer, but we cannot capture `id` in a fn.
    // Instead we use a lookup table matching the known built-in IDs.
    match id {
        "session-status" => || Box::new(session_status::SessionStatusWidget::new()),
        "working-dir" => working_dir::WorkingDirWidget::create,
        "api-usage" => api_usage::create,
        "state-history" => || {
            Box::new(PlaceholderWidget {
                widget_id: "state-history",
            })
        },
        "clock" => || Box::new(PlaceholderWidget { widget_id: "clock" }),
        "spacer" => || {
            Box::new(PlaceholderWidget {
                widget_id: "spacer",
            })
        },
        _ => || {
            Box::new(PlaceholderWidget {
                widget_id: "unknown",
            })
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Session;

    // -- Widget trait implementation tests --

    struct MockWidget;

    impl Widget for MockWidget {
        fn render(&self, width: u16, _context: &WidgetContext) -> Line<'_> {
            Line::raw(format!("mock:{width}"))
        }
        fn id(&self) -> &'static str {
            "mock"
        }
        fn min_width(&self) -> u16 {
            8
        }
    }

    #[test]
    fn test_mock_widget_implements_trait() {
        let w: Box<dyn Widget> = Box::new(MockWidget);
        assert_eq!(w.id(), "mock");
        assert_eq!(w.min_width(), 8);
    }

    #[test]
    fn test_mock_widget_render_returns_valid_line() {
        let sessions: Vec<Session> = vec![];
        let ctx = WidgetContext::new(&sessions);
        let w = MockWidget;
        let line = w.render(40, &ctx);
        let text = line.to_string();
        assert_eq!(text, "mock:40");
    }

    // -- WidgetRegistry tests --

    #[test]
    fn test_registry_creates_known_widgets() {
        let reg = WidgetRegistry::new();
        for id in &[
            "session-status",
            "working-dir",
            "api-usage",
            "state-history",
            "clock",
            "spacer",
        ] {
            let widget = reg.create(id);
            assert!(widget.is_some(), "expected factory for '{id}'");
            let widget = widget.expect("already checked");
            assert_eq!(widget.id(), *id);
        }
    }

    #[test]
    fn test_registry_returns_none_for_unknown() {
        let reg = WidgetRegistry::new();
        assert!(reg.create("nonexistent").is_none());
        assert!(reg.create("").is_none());
    }

    #[test]
    fn test_registry_available_ids_contains_all_builtins() {
        let reg = WidgetRegistry::new();
        let ids = reg.available_ids();
        assert_eq!(ids.len(), 6);
        for expected in &[
            "session-status",
            "working-dir",
            "api-usage",
            "state-history",
            "clock",
            "spacer",
        ] {
            assert!(
                ids.contains(expected),
                "missing '{expected}' in available_ids"
            );
        }
    }

    #[test]
    fn test_registry_register_custom_widget() {
        let mut reg = WidgetRegistry::new();
        fn custom_factory() -> Box<dyn Widget> {
            Box::new(MockWidget)
        }
        reg.register("mock", custom_factory);
        let w = reg.create("mock").expect("custom widget registered");
        assert_eq!(w.id(), "mock");
    }

    #[test]
    fn test_registry_register_overwrites_existing() {
        let mut reg = WidgetRegistry::new();
        fn custom_clock() -> Box<dyn Widget> {
            Box::new(MockWidget)
        }
        reg.register("clock", custom_clock);
        let w = reg.create("clock").expect("overwritten factory");
        // Now returns MockWidget instead of PlaceholderWidget
        assert_eq!(w.id(), "mock");
    }

    #[test]
    fn test_registry_default_trait() {
        let reg = WidgetRegistry::default();
        assert_eq!(reg.available_ids().len(), 6);
    }

    // -- Placeholder widget tests --

    #[test]
    fn test_placeholder_widget_render() {
        let sessions: Vec<Session> = vec![];
        let ctx = WidgetContext::new(&sessions);
        let w = PlaceholderWidget { widget_id: "clock" };
        let line = w.render(80, &ctx);
        assert_eq!(line.to_string(), "[clock]");
    }

    #[test]
    fn test_placeholder_widget_min_width() {
        let w = PlaceholderWidget { widget_id: "clock" };
        // "clock" = 5 chars + 2 brackets + 1 padding = 8
        assert_eq!(w.min_width(), 8);
    }

    #[test]
    fn test_placeholder_widget_id() {
        let w = PlaceholderWidget {
            widget_id: "spacer",
        };
        assert_eq!(w.id(), "spacer");
    }

    #[test]
    fn test_placeholder_widget_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<PlaceholderWidget>();
    }

    #[test]
    fn test_widget_trait_object_is_send_sync() {
        fn assert_send_sync<T: Send + Sync + ?Sized>() {}
        assert_send_sync::<dyn Widget>();
    }

    #[test]
    fn test_registry_create_returns_independent_instances() {
        let reg = WidgetRegistry::new();
        let w1 = reg.create("clock").expect("clock exists");
        let w2 = reg.create("clock").expect("clock exists");
        // Both are independent instances
        assert_eq!(w1.id(), w2.id());
        assert_eq!(w1.min_width(), w2.min_width());
    }
}
