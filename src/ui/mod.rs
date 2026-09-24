pub mod connect;
pub mod header;
pub mod network_page;
pub mod networks;
pub mod vpn_add_page;
pub mod vpn_details_page;
pub mod vpn_list;
pub mod wired_devices;
pub mod wired_page;

use adw::prelude::*;
use adw::{Application, ApplicationWindow};
use gtk::{Box as GtkBox, Image, Label, Orientation, ScrolledWindow, Stack, pango::EllipsizeMode};
use std::cell::Cell;
use std::rc::Rc;
use std::sync::Arc;
use tokio::sync::Notify;

type Callback = Rc<dyn Fn()>;
type CallbackCell = Rc<std::cell::RefCell<Option<Callback>>>;

#[macro_export]
macro_rules! strong_clone {
    ( ($( $x:ident ),*) $y:expr ) => {
        {
            $(let $x = $x.clone();)*
            $y
        }
    };
}

macro_rules! page {
    ($child:expr, $title:expr, $tag:expr) => {{
        let toolbar_view = adw::ToolbarView::new();
        let page = adw::NavigationPage::new(&toolbar_view, $title);
        toolbar_view.add_top_bar(&adw::HeaderBar::new());
        toolbar_view.set_content(Some(&$child));
        page.set_tag(Some($tag));
        page
    }};
}

pub fn freq_to_band(freq: u32) -> Option<&'static str> {
    match freq {
        2400..=2500 => Some("2.4GHz"),
        5150..=5925 => Some("5GHz"),
        5926..=7125 => Some("6GHz"),
        _ => None,
    }
}

pub fn build_ui(app: &Application) {
    let win = ApplicationWindow::new(app);
    win.set_title(Some(""));
    win.set_default_size(450, 600);

    let nav_view = adw::NavigationView::new();

    let toolbar_view = adw::ToolbarView::new();
    let main_page = adw::NavigationPage::new(&toolbar_view, "");
    nav_view.add(&main_page);

    let status = Label::new(None);
    status.set_xalign(0.0);
    status.set_ellipsize(EllipsizeMode::End);
    status.set_max_width_chars(36);

    let conn_icon = Image::from_icon_name("network-offline-symbolic");
    conn_icon.add_css_class("conn-status-icon");

    let conn_name = Label::new(Some("Disconnected"));
    conn_name.set_ellipsize(EllipsizeMode::End);
    conn_name.set_max_width_chars(20);
    conn_name.add_css_class("conn-status-name");

    let scan_spinner = adw::Spinner::new();
    scan_spinner.set_visible(false);

    let list_container = GtkBox::new(Orientation::Vertical, 24);
    let stack = Stack::new();
    let is_scanning = Rc::new(Cell::new(false));
    list_container.set_margin_bottom(24);
    list_container.set_margin_top(4);
    list_container.set_margin_start(24);
    list_container.set_margin_end(24);

    let spinner = adw::Spinner::new();
    spinner.set_halign(gtk::Align::Center);
    spinner.set_valign(gtk::Align::Center);
    spinner.set_property("width-request", 24i32);
    spinner.set_property("height-request", 24i32);
    spinner.add_css_class("loading-spinner");

    stack.add_named(&spinner, Some("loading"));
    stack.set_visible_child_name("loading");

    let networks_scroller = ScrolledWindow::new();
    networks_scroller.set_vexpand(true);
    networks_scroller.set_policy(gtk::PolicyType::Never, gtk::PolicyType::Automatic);
    networks_scroller.set_child(Some(&list_container));

    stack.add_named(&networks_scroller, Some("networks"));

    stack.set_vexpand(true);
    toolbar_view.set_content(Some(&stack));

    win.set_content(Some(&nav_view));
    win.show();

    glib::MainContext::default().spawn_local(async move {
        match nmrs::NetworkManager::new().await {
            Ok(nm) => {
                let nm = Rc::new(nm);

                let details_page = Rc::new(network_page::NetworkPage::new(&stack));
                let details_scroller = ScrolledWindow::new();
                details_scroller.set_policy(gtk::PolicyType::Never, gtk::PolicyType::Automatic);
                details_scroller.set_child(Some(details_page.widget()));
                nav_view.add(&page!(details_scroller, "Details", "details"));

                let wired_details_page = Rc::new(wired_page::WiredPage::new(&stack));
                let wired_details_scroller = ScrolledWindow::new();
                wired_details_scroller
                    .set_policy(gtk::PolicyType::Never, gtk::PolicyType::Automatic);
                wired_details_scroller.set_child(Some(wired_details_page.widget()));
                nav_view.add(&page!(wired_details_scroller, "Details", "wired-details"));

                let vpn_details_page = Rc::new(vpn_details_page::VpnDetailsPage::new(&stack));
                let vpn_details_scroller = ScrolledWindow::new();
                vpn_details_scroller.set_policy(gtk::PolicyType::Never, gtk::PolicyType::Automatic);
                vpn_details_scroller.set_child(Some(vpn_details_page.widget()));
                nav_view.add(&page!(vpn_details_scroller, "Details", "vpn-details"));

                let vpn_add = vpn_add_page::VpnAddPage::new(&stack, &win);
                let vpn_add_scroller = ScrolledWindow::new();
                vpn_add_scroller.set_policy(gtk::PolicyType::Never, gtk::PolicyType::Automatic);
                vpn_add_scroller.set_child(Some(vpn_add.widget()));
                nav_view.add(&page!(vpn_add_scroller, "Add VPN", "vpn-add"));

                let conn_icon = conn_icon.clone();
                let conn_name = conn_name.clone();
                let scan_spinner = scan_spinner.clone();

                let on_success: Rc<dyn Fn()> = {
                    let on_success_cell: CallbackCell = Rc::new(std::cell::RefCell::new(None));
                    let parent_window = win.clone();
                    let callback = on_success_cell.borrow().as_ref().map(|cb| cb.clone());
                    let refresh_ctx = Rc::new(networks::NetworksContext {
                        nm: nm.clone(),
                        on_success: callback.unwrap_or_else(|| Rc::new(|| {})),
                        status: status.clone(),
                        conn_icon: conn_icon.clone(),
                        conn_name: conn_name.clone(),
                        scan_spinner: scan_spinner.clone(),
                        stack: stack.clone(),
                        nav_view: nav_view.clone(),
                        parent_window: parent_window.clone(),
                        details_page: details_page.clone(),
                        wired_details_page: wired_details_page.clone(),
                        vpn_details_page: vpn_details_page.clone(),
                    });
                    let callback = Rc::new(strong_clone!((list_container, is_scanning) move || {
                        glib::spawn_future_local(strong_clone!(
                            (refresh_ctx, list_container, is_scanning) async move {
                            header::refresh_networks(
                                refresh_ctx,
                                &list_container,
                                &is_scanning,
                            )
                            .await;
                        }));
                    })) as Rc<dyn Fn()>;

                    *on_success_cell.borrow_mut() = Some(callback.clone());

                    callback
                };

                let ctx = Rc::new(networks::NetworksContext {
                    nm: nm.clone(),
                    on_success: on_success.clone(),
                    status: status.clone(),
                    conn_icon: conn_icon.clone(),
                    conn_name: conn_name.clone(),
                    scan_spinner: scan_spinner.clone(),
                    stack: stack.clone(),
                    nav_view: nav_view.clone(),
                    parent_window: win.clone(),
                    details_page: details_page.clone(),
                    wired_details_page,
                    vpn_details_page: vpn_details_page.clone(),
                });

                details_page.set_on_success(on_success.clone());
                vpn_details_page.set_on_success(on_success.clone());
                vpn_add.set_on_success(on_success);

                let header =
                    header::build_header(ctx.clone(), &list_container, is_scanning.clone());
                toolbar_view.add_top_bar(&header);

                {
                    let nm_device_monitor = nm.clone();
                    let device_notify = Arc::new(Notify::new());

                    let notify = device_notify.clone();
                    glib::MainContext::default().spawn_local(async move {
                        loop {
                            let notify = notify.clone();
                            let result = nm_device_monitor
                                .monitor_device_changes(move || {
                                    notify.notify_one();
                                })
                                .await;

                            if let Err(e) = result {
                                eprintln!("Device monitoring error: {}, restarting in 5s...", e)
                            }
                            glib::timeout_future_seconds(5).await;
                        }
                    });

                    let list_container_device = list_container.clone();
                    let is_scanning_device = is_scanning.clone();
                    let ctx_device = ctx.clone();
                    glib::MainContext::default().spawn_local(async move {
                        loop {
                            device_notify.notified().await;
                            glib::timeout_future_seconds(3).await;

                            let current_page = ctx_device.stack.visible_child_name();
                            let on_networks_page = current_page.as_deref() == Some("networks");

                            if !is_scanning_device.get() && on_networks_page {
                                header::refresh_networks_no_scan(
                                    ctx_device.clone(),
                                    &list_container_device,
                                    &is_scanning_device,
                                )
                                .await;
                            }
                        }
                    });
                }

                {
                    let nm_network_monitor = nm.clone();
                    let network_notify = Arc::new(Notify::new());

                    let notify = network_notify.clone();
                    glib::MainContext::default().spawn_local(async move {
                        loop {
                            let notify = notify.clone();
                            let result = nm_network_monitor
                                .monitor_network_changes(move || {
                                    notify.notify_one();
                                })
                                .await;

                            if let Err(e) = result {
                                eprintln!("Network monitoring error: {}, restarting in 5s...", e)
                            }
                            glib::timeout_future_seconds(5).await;
                        }
                    });

                    let list_container_network = list_container.clone();
                    let is_scanning_network = is_scanning.clone();
                    let ctx_network = ctx.clone();
                    glib::MainContext::default().spawn_local(async move {
                        loop {
                            network_notify.notified().await;
                            glib::timeout_future_seconds(8).await;

                            let current_page = ctx_network.stack.visible_child_name();
                            let on_networks_page = current_page.as_deref() == Some("networks");

                            if !is_scanning_network.get() && on_networks_page {
                                header::refresh_networks_no_scan(
                                    ctx_network.clone(),
                                    &list_container_network,
                                    &is_scanning_network,
                                )
                                .await;
                            }
                        }
                    });
                }
            }
            Err(err) => {
                status.set_text(&format!("Failed to initialize: {err}"));
            }
        }
    });
}
