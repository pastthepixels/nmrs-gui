use gtk::prelude::*;
use gtk::{Align, Box, Button, Label, Orientation};

const CUSTOM_INDEX: u32 = 0;

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

        let back = Button::with_label("← Back");
        back.add_css_class("back-button");
        back.set_halign(Align::Start);
        back.set_cursor_from_name(Some("pointer"));
        {
            let stack = stack.clone();
            back.connect_clicked(move |_| {
                stack.set_visible_child_name("networks");
            });
        }
        root.append(&back);

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
