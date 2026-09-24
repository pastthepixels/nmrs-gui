use gtk::prelude::*;
use gtk::{Align, Box, Button, Label, Orientation};

pub struct SettingsPage {
    root: gtk::Box,
}

impl SettingsPage {
    pub fn new(stack: &gtk::Stack, _window: &adw::ApplicationWindow) -> Self {
        let root = Box::new(Orientation::Vertical, 12);
        root.add_css_class("settings-page");
        root.set_margin_top(12);
        root.set_margin_bottom(12);
        root.set_margin_start(16);
        root.set_margin_end(16);

        let title = Label::new(Some("Settings"));
        title.add_css_class("section-header");
        title.set_halign(Align::Start);
        root.append(&title);

        Self { root }
    }

    pub fn widget(&self) -> &gtk::Box {
        &self.root
    }
}
