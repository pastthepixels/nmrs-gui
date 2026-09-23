use adw::prelude::PreferencesGroupExt;
use adw::prelude::*;
use gtk::Align;
use gtk::GestureClick;
use gtk::prelude::*;
use gtk::{Box, Image, Label, Orientation};
use nmrs::models;
use std::rc::Rc;

use crate::ui::networks::NetworksContext;
use crate::ui::wired_page::WiredPage;

pub struct WiredDeviceRowController {
    pub row: adw::ActionRow,
    pub arrow: gtk::Button,
    pub ctx: Rc<NetworksContext>,
    pub device: models::Device,
    pub details_page: Rc<WiredPage>,
}

impl WiredDeviceRowController {
    pub fn new(
        row: adw::ActionRow,
        arrow: gtk::Button,
        ctx: Rc<NetworksContext>,
        device: models::Device,
        details_page: Rc<WiredPage>,
    ) -> Self {
        Self {
            row,
            arrow,
            ctx,
            device,
            details_page,
        }
    }

    pub fn attach(&self) {
        self.attach_arrow();
        self.attach_row_double();
    }

    fn attach_arrow(&self) {
        let click = GestureClick::new();

        let device = self.device.clone();
        let stack = self.ctx.stack.clone();
        let page = self.details_page.clone();

        click.connect_pressed(move |_, _, _, _| {
            let device_c = device.clone();
            let stack_c = stack.clone();
            let page_c = page.clone();

            glib::MainContext::default().spawn_local(async move {
                page_c.update(&device_c);
                stack_c.set_visible_child_name("wired-details");
            });
        });

        self.arrow.add_controller(click);
    }

    fn attach_row_double(&self) {
        let click = GestureClick::new();

        let ctx = self.ctx.clone();
        let device = self.device.clone();
        let interface = device.interface.clone();

        let status = ctx.status.clone();
        let window = ctx.parent_window.clone();
        let on_success = ctx.on_success.clone();

        click.connect_pressed(move |_, n, _, _| {
            if n != 2 {
                return;
            }

            status.set_text(&format!("Connecting to {interface}..."));

            let nm_c = ctx.nm.clone();
            let status_c = status.clone();
            let window_c = window.clone();
            let on_success_c = on_success.clone();

            glib::MainContext::default().spawn_local(async move {
                window_c.set_sensitive(false);
                match nm_c.connect_wired().await {
                    Ok(_) => {
                        status_c.set_text("");
                        on_success_c();
                    }
                    Err(e) => status_c.set_text(&format!("Failed to connect: {e}")),
                }
                window_c.set_sensitive(true);
                status_c.set_text("");
            });
        });

        self.row.add_controller(click);
    }
}

pub fn wired_devices_view(
    ctx: Rc<NetworksContext>,
    devices: &[models::Device],
    details_page: Rc<WiredPage>,
) -> adw::PreferencesGroup {
    let list = adw::PreferencesGroup::new();
    list.set_title("Wired");

    for device in devices {
        let row = adw::ActionRow::new();

        row.set_title(&format!("{} ({})", device.interface, device.device_type));
        match device.state {
            models::DeviceState::Activated => Some("Connected"),
            models::DeviceState::Disconnected => Some("Disconnected"),
            models::DeviceState::Unavailable => Some("Unavailable"),
            models::DeviceState::Failed => Some("Failed"),
            // Hide transitional states (Unmanaged, Prepare, Config, etc)
            _ => None,
        }
        .map(|s| row.set_subtitle(s));

        let icon = Image::from_icon_name("network-wired-symbolic");
        icon.add_css_class("wired-icon");
        row.add_prefix(&icon);

        let arrow = gtk::Button::from_icon_name("go-next-symbolic");
        arrow.set_valign(Align::Center);
        arrow.set_vexpand(false);
        arrow.add_css_class("flat");
        row.add_suffix(&arrow);

        let controller = WiredDeviceRowController::new(
            row.clone(),
            arrow.clone(),
            ctx.clone(),
            device.clone(),
            details_page.clone(),
        );

        controller.attach();

        list.add(&row);
    }
    list
}
