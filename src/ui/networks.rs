use adw::prelude::*;
use anyhow::Result;
use gtk::Align;
use gtk::{Image, Label};
use nmrs::models::WifiSecurity;
use nmrs::{NetworkManager, models};
use std::collections::HashSet;
use std::rc::Rc;

use crate::strong_clone;
use crate::ui::connect;
use crate::ui::network_page::NetworkPage;

pub struct NetworkRowController {
    pub row: adw::ActionRow,
    pub arrow: gtk::Button,
    pub ctx: Rc<NetworksContext>,
    pub net: models::Network,
    pub details_page: Rc<NetworkPage>,
}

pub struct NetworksContext {
    pub nm: Rc<NetworkManager>,
    pub on_success: Rc<dyn Fn()>,
    pub status: Label,
    pub conn_icon: Image,
    pub conn_name: Label,
    pub scan_spinner: adw::Spinner,
    pub stack: gtk::Stack,
    pub nav_view: adw::NavigationView,
    pub parent_window: adw::ApplicationWindow,
    pub details_page: Rc<NetworkPage>,
    pub wired_details_page: Rc<crate::ui::wired_page::WiredPage>,
    pub vpn_details_page: Rc<crate::ui::vpn_details_page::VpnDetailsPage>,
}

impl NetworksContext {
    pub async fn new(
        on_success: Rc<dyn Fn()>,
        status: &Label,
        conn_icon: &Image,
        conn_name: &Label,
        scan_spinner: &adw::Spinner,
        stack: &gtk::Stack,
        nav_view: &adw::NavigationView,
        parent_window: &adw::ApplicationWindow,
        details_page: Rc<NetworkPage>,
        wired_details_page: Rc<crate::ui::wired_page::WiredPage>,
        vpn_details_page: Rc<crate::ui::vpn_details_page::VpnDetailsPage>,
    ) -> Result<Self> {
        let nm = Rc::new(NetworkManager::new().await?);

        Ok(Self {
            nm,
            on_success,
            status: status.clone(),
            conn_icon: conn_icon.clone(),
            conn_name: conn_name.clone(),
            scan_spinner: scan_spinner.clone(),
            stack: stack.clone(),
            nav_view: nav_view.clone(),
            parent_window: parent_window.clone(),
            details_page,
            wired_details_page,
            vpn_details_page,
        })
    }
}

impl NetworkRowController {
    pub fn new(
        row: adw::ActionRow,
        arrow: gtk::Button,
        ctx: Rc<NetworksContext>,
        net: models::Network,
        details_page: Rc<NetworkPage>,
    ) -> Self {
        Self {
            row,
            arrow,
            ctx,
            net,
            details_page,
        }
    }

    pub fn attach(&self) {
        self.attach_arrow();
        self.attach_row_double();
    }

    fn attach_arrow(&self) {
        let ctx = self.ctx.clone();
        let net = self.net.clone();
        let page = self.details_page.clone();
        let nav_view = self.ctx.nav_view.clone();

        self.arrow.connect_clicked(move |_| {
            glib::MainContext::default().spawn_local(
                strong_clone!((ctx, net, page, nav_view) async move {
                    if let Ok(info) = ctx.nm.show_details(&net).await {
                        page.update(&info);

                        if let Ok(aps) = ctx.nm.list_access_points(None).await {
                            let best = aps
                                .iter()
                                .filter(|ap| ap.ssid == net.ssid)
                                .max_by_key(|ap| ap.strength);
                            if let Some(ap) = best {
                                page.enrich_with_ap(ap);
                            }
                        }

                        nav_view.push_by_tag("details");
                    }
                }),
            );
        });
    }

    fn attach_row_double(&self) {
        let ctx = self.ctx.clone();
        let net = self.net.clone();
        let ssid = net.ssid.clone();
        let secured = net.secured;
        let is_eap = net.is_eap;

        let status = ctx.status.clone();
        let window = ctx.parent_window.clone();
        let on_success = ctx.on_success.clone();

        self.row.set_activatable(true);
        self.row.connect_activated(move |row| {
            row.set_subtitle("Connecting…");

            let ssid_c = ssid.clone();
            let nm_c = ctx.nm.clone();
            let status_c = status.clone();
            let window_c = window.clone();
            let on_success_c = on_success.clone();
            let row = row.clone();

            glib::MainContext::default().spawn_local(async move {
                if secured {
                    let have = nm_c.has_saved_connection(&ssid_c).await.unwrap_or(false);

                    if have {
                        row.set_subtitle("Connecting…");
                        window_c.set_sensitive(false);
                        let creds = WifiSecurity::WpaPsk { psk: "".into() };
                        match nm_c.connect(&ssid_c, None, creds).await {
                            Ok(_) => {
                                on_success_c();
                            }
                            Err(e) => status_c.set_text(&format!("Failed to connect: {e}")),
                        }
                        window_c.set_sensitive(true);
                    } else {
                        connect::connect_modal(
                            nm_c.clone(),
                            &window_c,
                            &ssid_c,
                            is_eap,
                            on_success_c.clone(),
                        );
                    }
                } else {
                    row.set_subtitle("Connecting…");
                    window_c.set_sensitive(false);
                    let creds = WifiSecurity::Open;
                    match nm_c.connect(&ssid_c, None, creds).await {
                        Ok(_) => {
                            status_c.set_text("");
                            on_success_c();
                        }
                        Err(e) => status_c.set_text(&format!("Failed to connect: {e}")),
                    }

                    window_c.set_sensitive(true);
                }

                row.set_subtitle("");
                status_c.set_text("");
            });
        });
    }
}

pub fn networks_view(
    ctx: Rc<NetworksContext>,
    networks: &[models::Network],
    current_ssid: Option<&str>,
    current_band: Option<&str>,
    saved_ssids: &HashSet<String>,
) -> adw::PreferencesGroup {
    let conn_threshold = 75;
    let list = adw::PreferencesGroup::new();
    list.set_title("Wireless");

    let mut sorted_networks: Vec<_> = networks
        .iter()
        .filter(|net| !net.ssid.trim().is_empty())
        .cloned()
        .collect();

    sorted_networks.sort_by(|a, b| {
        let a_connected = is_current_network(a, current_ssid, current_band);
        let b_connected = is_current_network(b, current_ssid, current_band);

        match (a_connected, b_connected) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => b.strength.unwrap_or(0).cmp(&a.strength.unwrap_or(0)),
        }
    });

    for net in sorted_networks {
        let row = adw::ActionRow::new();

        let display_name = match net.frequency.and_then(crate::ui::freq_to_band) {
            Some(band) => format!("{} ({band})", net.ssid),
            None => net.ssid.clone(),
        };

        row.set_title(&display_name);

        if is_current_network(&net, current_ssid, current_band) {
            row.set_subtitle("Connected");
        } else if saved_ssids.contains(&net.ssid) {
            row.set_subtitle("Saved");
        }

        if let Some(s) = net.strength {
            let icon_name = if net.secured {
                "network-wireless-encrypted-symbolic"
            } else {
                "network-wireless-signal-excellent-symbolic"
            };

            let image = Image::from_icon_name(icon_name);
            if net.secured {
                image.add_css_class("wifi-secure");
            } else {
                image.add_css_class("wifi-open");
            }

            let strength_label = Label::new(Some(&format!("{s}%")));
            row.add_prefix(&strength_label);
            row.add_prefix(&image);

            if s >= conn_threshold {
                strength_label.add_css_class("success");
            } else if s > 65 {
                strength_label.add_css_class("warning");
            } else {
                strength_label.add_css_class("error");
            }

            strength_label.add_css_class("dimmed");
            strength_label.add_css_class("heading");
        }

        let arrow = gtk::Button::from_icon_name("go-next-symbolic");
        arrow.set_valign(Align::Center);
        arrow.set_vexpand(false);
        arrow.set_halign(Align::End);
        arrow.add_css_class("flat");
        arrow.set_cursor_from_name(Some("pointer"));
        row.add_suffix(&arrow);

        let controller = NetworkRowController::new(
            row.clone(),
            arrow.clone(),
            ctx.clone(),
            net.clone(),
            ctx.details_page.clone(),
        );

        controller.attach();

        list.add(&row);
    }
    list
}

fn is_current_network(
    net: &models::Network,
    current_ssid: Option<&str>,
    current_band: Option<&str>,
) -> bool {
    let ssid = match current_ssid {
        Some(s) => s,
        None => return false,
    };

    if net.ssid != ssid {
        return false;
    }

    if let Some(band) = current_band {
        let net_band = net.frequency.and_then(crate::ui::freq_to_band);

        return net_band == Some(band);
    }

    true
}
