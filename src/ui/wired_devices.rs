use adw::prelude::PreferencesGroupExt;
use adw::prelude::*;
use gtk::Align;
use gtk::GestureClick;
use gtk::Image;
use nmrs::models;
use std::rc::Rc;

use crate::strong_clone;
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
        let device = self.device.clone();
        let nav_view = self.ctx.nav_view.clone();
        let page = self.details_page.clone();

        self.arrow.connect_clicked(move |_| {
            glib::MainContext::default().spawn_local(
                strong_clone!((device, page, nav_view) async move {
                    page.update(&device);
                    nav_view.push_by_tag("wired-details");
                }),
            );
        });
    }

    fn attach_row_double(&self) {
        let click = GestureClick::new();

        let ctx = self.ctx.clone();

        let status = ctx.status.clone();
        let window = ctx.parent_window.clone();
        let on_success = ctx.on_success.clone();

        self.row.set_activatable(true);
        self.row.connect_activated(move |row| {
            row.set_subtitle("Connecting…");

            let nm_c = ctx.nm.clone();
            let status_c = status.clone();
            let window_c = window.clone();
            let on_success_c = on_success.clone();
            let row = row.clone();

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
                row.set_subtitle("");
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
        if let Some(s) = match device.state {
            models::DeviceState::Activated => Some("Connected"),
            models::DeviceState::Disconnected => Some("Disconnected"),
            models::DeviceState::Unavailable => Some("Unavailable"),
            models::DeviceState::Failed => Some("Failed"),
            // Hide transitional states (Unmanaged, Prepare, Config, etc)
            _ => None,
        } {
            row.set_subtitle(s)
        }

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
