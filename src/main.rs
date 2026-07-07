use gtk4 as gtk;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::{
    Application, ApplicationWindow, Box, Entry, Orientation, ListView, ScrolledWindow,
    CssProvider, style_context_add_provider_for_display, Label, PolicyType,
    SignalListItemFactory, SingleSelection, Image
};
use gtk::gio;
use gtk::gdk;
use gdk::Display;
use std::rc::Rc;
use std::cell::RefCell;

use tracker::prelude::*;
use tracker::SparqlConnection;

// Custom GObject for holding search results
mod imp {
    use gtk4 as gtk;
    use gtk::gio;
    use gtk::glib;
    use gtk::subclass::prelude::*;
    use std::cell::RefCell;

    #[derive(Default)]
    pub struct SearchResult {
        pub title: RefCell<String>,
        pub subtitle: RefCell<String>,
        pub icon_name: RefCell<String>,
        pub app_info: RefCell<Option<gio::AppInfo>>,
        pub file_uri: RefCell<Option<String>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for SearchResult {
        const NAME: &'static str = "SearchResult";
        type Type = super::SearchResult;
        type ParentType = glib::Object;
    }

    impl ObjectImpl for SearchResult {}
}

use gtk::glib;

glib::wrapper! {
    pub struct SearchResult(ObjectSubclass<imp::SearchResult>);
}

impl SearchResult {
    pub fn new_app(title: &str, subtitle: &str, icon_name: &str, app_info: &gio::AppInfo) -> Self {
        let obj: Self = glib::Object::builder().build();
        *obj.imp().title.borrow_mut() = title.to_string();
        *obj.imp().subtitle.borrow_mut() = subtitle.to_string();
        *obj.imp().icon_name.borrow_mut() = icon_name.to_string();
        *obj.imp().app_info.borrow_mut() = Some(app_info.clone());
        obj
    }

    pub fn new_file(title: &str, subtitle: &str, icon_name: &str, file_uri: &str) -> Self {
        let obj: Self = glib::Object::builder().build();
        *obj.imp().title.borrow_mut() = title.to_string();
        *obj.imp().subtitle.borrow_mut() = subtitle.to_string();
        *obj.imp().icon_name.borrow_mut() = icon_name.to_string();
        *obj.imp().file_uri.borrow_mut() = Some(file_uri.to_string());
        obj
    }

    pub fn new_mock(title: &str, subtitle: &str, icon_name: &str) -> Self {
        let obj: Self = glib::Object::builder().build();
        *obj.imp().title.borrow_mut() = title.to_string();
        *obj.imp().subtitle.borrow_mut() = subtitle.to_string();
        *obj.imp().icon_name.borrow_mut() = icon_name.to_string();
        obj
    }

    pub fn title(&self) -> String {
        self.imp().title.borrow().clone()
    }

    pub fn subtitle(&self) -> String {
        self.imp().subtitle.borrow().clone()
    }

    pub fn icon_name(&self) -> String {
        self.imp().icon_name.borrow().clone()
    }
}

const CSS_DATA: &str = "
window {
    background: transparent;
}

.main-box {
    background-color: #242424;
    border-radius: 16px;
    border: 1px solid rgba(255, 255, 255, 0.12);
    box-shadow: 0 12px 40px rgba(0, 0, 0, 0.6);
    margin: 20px; /* Space for the shadow */
    padding: 12px;
}

.search-entry {
    background: rgba(255, 255, 255, 0.05);
    border: 1px solid rgba(255, 255, 255, 0.1);
    border-radius: 8px;
    font-size: 18px;
    color: #ffffff;
    padding: 12px;
    margin-bottom: 8px;
    caret-color: #3584e4;
}

.search-entry:focus {
    border-color: #3584e4;
    background: rgba(255, 255, 255, 0.08);
}

/* Remove default focus outlines */
listview {
    background: transparent;
    border: none;
    outline: none;
}

listitem {
    outline: none;
    border: none;
}

row {
    outline: none;
    border: none;
    padding: 10px 14px;
    border-radius: 8px;
    margin: 2px 0;
    color: #e0e0e0;
}

row:hover {
    background-color: rgba(255, 255, 255, 0.05);
}

row:selected {
    background-color: #3584e4;
    color: #ffffff;
    outline: none;
}

.row-title {
    font-size: 15px;
    font-weight: bold;
}

.row-subtitle {
    font-size: 12px;
    color: #909090;
}

row:selected .row-subtitle {
    color: #d0d0d0;
}

.row-icon {
    opacity: 0.9;
}

row:selected .row-icon {
    opacity: 1.0;
}
";

fn main() {
    let app = Application::builder()
        .application_id("com.sammy.LinuxSpotlight")
        .flags(gio::ApplicationFlags::HANDLES_COMMAND_LINE)
        .build();

    // Create the persistent Tracker SparqlConnection
    let tracker_conn = match SparqlConnection::bus_new("org.freedesktop.Tracker3.Miner.Files", None, None) {
        Ok(conn) => Some(conn),
        Err(e) => {
            eprintln!("Warning: Failed to connect to Tracker 3 Files Miner: {}", e);
            None
        }
    };

    // Application state: holds references to the window and search entry
    let state: Rc<RefCell<Option<(ApplicationWindow, Entry)>>> = Rc::new(RefCell::new(None));

    let tracker_conn_clone = tracker_conn.clone();
    let state_clone = state.clone();
    app.connect_activate(move |app| {
        if state_clone.borrow().is_none() {
            let (window, search_entry) = build_ui(app, &tracker_conn_clone);
            *state_clone.borrow_mut() = Some((window, search_entry));
        }
    });

    let state_clone2 = state.clone();
    app.connect_command_line(move |app, _cmd_line| {
        let mut is_first = false;
        if state_clone2.borrow().is_none() {
            app.activate();
            is_first = true;
        }

        if let Some((window, search_entry)) = &*state_clone2.borrow() {
            if is_first {
                window.present();
            } else if window.is_visible() {
                window.hide();
            } else {
                search_entry.set_text("");
                window.present();
            }
        }
        0.into()
    });

    app.run();
}

fn build_ui(app: &Application, tracker_conn: &Option<SparqlConnection>) -> (ApplicationWindow, Entry) {
    // 1. Create Window
    let window = ApplicationWindow::builder()
        .application(app)
        .title("Linux Spotlight")
        .default_width(650)
        .resizable(false)
        .decorated(false)
        .build();

    // 2. Setup CSS
    let provider = CssProvider::new();
    provider.load_from_data(CSS_DATA);
    if let Some(display) = Display::default() {
        style_context_add_provider_for_display(
            &display,
            &provider,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    }

    // 3. Create UI Layout
    let main_box = Box::builder()
        .orientation(Orientation::Vertical)
        .build();
    main_box.add_css_class("main-box");

    // Search bar
    let search_entry = Entry::builder()
        .placeholder_text("Search files and applications...")
        .build();
    search_entry.add_css_class("search-entry");
    main_box.append(&search_entry);

    // Scrolled window for results list
    let scrolled_window = ScrolledWindow::builder()
        .hscrollbar_policy(PolicyType::Never)
        .vscrollbar_policy(PolicyType::Automatic)
        .min_content_height(0)
        .max_content_height(400)
        .propagate_natural_height(true) // key to dynamic height!
        .build();

    // Reactive data list store
    let list_store = gio::ListStore::new::<SearchResult>();

    // Connect to items-changed signal to dynamically resize/shrink the window to fit the visible rows perfectly
    let window_clone_size = window.clone();
    let scrolled_window_clone = scrolled_window.clone();
    list_store.connect_items_changed(move |store, _, _, _| {
        let has_items = store.n_items() > 0;
        scrolled_window_clone.set_visible(has_items);

        let window_clone_size = window_clone_size.clone();
        glib::idle_add_local_once(move || {
            if window_clone_size.is_visible() {
                window_clone_size.set_default_size(650, -1);
            }
        });
    });

    // Single selection model
    let selection_model = SingleSelection::new(Some(list_store.clone()));

    // ListView factory
    let factory = SignalListItemFactory::new();

    factory.connect_setup(move |_, list_item| {
        let row_container = Box::builder()
            .orientation(Orientation::Horizontal)
            .spacing(12)
            .build();

        let icon_image = Image::builder()
            .pixel_size(32)
            .halign(gtk::Align::Center)
            .valign(gtk::Align::Center)
            .build();
        icon_image.add_css_class("row-icon");

        let text_box = Box::builder()
            .orientation(Orientation::Vertical)
            .spacing(1)
            .build();

        let title_label = Label::builder()
            .halign(gtk::Align::Start)
            .build();
        title_label.add_css_class("row-title");

        let subtitle_label = Label::builder()
            .halign(gtk::Align::Start)
            .build();
        subtitle_label.add_css_class("row-subtitle");

        text_box.append(&title_label);
        text_box.append(&subtitle_label);

        row_container.append(&icon_image);
        row_container.append(&text_box);

        list_item.set_child(Some(&row_container));
    });

    factory.connect_bind(move |_, list_item| {
        let item = list_item.item().unwrap();
        let search_result = item.downcast_ref::<SearchResult>().unwrap();

        let row_container = list_item.child().unwrap().downcast::<Box>().unwrap();
        let icon_image = row_container.first_child().unwrap().downcast::<Image>().unwrap();
        let text_box = icon_image.next_sibling().unwrap().downcast::<Box>().unwrap();
        let title_label = text_box.first_child().unwrap().downcast::<Label>().unwrap();
        let subtitle_label = title_label.next_sibling().unwrap().downcast::<Label>().unwrap();

        title_label.set_text(&search_result.title());
        subtitle_label.set_text(&search_result.subtitle());
        icon_image.set_icon_name(Some(&search_result.icon_name()));
    });

    let list_view = ListView::new(Some(selection_model.clone()), Some(factory));
    
    scrolled_window.set_child(Some(&list_view));
    main_box.append(&scrolled_window);

    window.set_child(Some(&main_box));

    // 4. Connect keyboard navigation redirect on the search entry
    let selection_model_clone = selection_model.clone();
    let list_store_clone = list_store.clone();
    let list_view_clone = list_view.clone();

    let entry_controller = gtk::EventControllerKey::new();
    entry_controller.connect_key_pressed(move |_, key, _, _| {
        match key {
            gdk::Key::Down => {
                // Instantly transfer focus to the results list natively
                if list_store_clone.n_items() > 0 {
                    list_view_clone.grab_focus();
                    selection_model_clone.set_selected(0);
                }
                gtk::glib::Propagation::Stop
            }
            _ => gtk::glib::Propagation::Proceed
        }
    });
    search_entry.add_controller(entry_controller);

    // 4.5 Connect Enter key directly to the search entry activation
    let selection_model_clone3 = selection_model.clone();
    let list_store_clone3 = list_store.clone();
    let window_clone4 = window.clone();
    let search_entry_clone4 = search_entry.clone();
    search_entry.connect_activate(move |_| {
        let current_selected = selection_model_clone3.selected();
        let index_to_activate = if current_selected == gtk::INVALID_LIST_POSITION {
            0
        } else {
            current_selected
        };

        if list_store_clone3.n_items() > index_to_activate {
            if let Some(item) = list_store_clone3.item(index_to_activate) {
                if let Ok(search_result) = item.downcast::<SearchResult>() {
                    execute_action(&search_result, &window_clone4, &search_entry_clone4);
                }
            }
        }
    });

    // 5. Connect keyboard navigation on the ListView itself
    let search_entry_clone2 = search_entry.clone();
    let selection_model_clone2 = selection_model.clone();
    let list_view_controller = gtk::EventControllerKey::new();
    list_view_controller.connect_key_pressed(move |_, key, _, _| {
        if key == gdk::Key::Up {
            let selected = selection_model_clone2.selected();
            if selected == 0 || selected == gtk::INVALID_LIST_POSITION {
                // Focus back to search entry smoothly
                search_entry_clone2.grab_focus();
                let len = search_entry_clone2.text_length();
                search_entry_clone2.set_position(len as i32);
                return gtk::glib::Propagation::Stop;
            }
        }
        gtk::glib::Propagation::Proceed
    });
    list_view.add_controller(list_view_controller);

    // 6. Connect ListView activation (Enter / Click)
    let list_store_clone2 = list_store.clone();
    let window_clone2 = window.clone();
    let search_entry_clone3 = search_entry.clone();
    list_view.connect_activate(move |_, position| {
        if let Some(item) = list_store_clone2.item(position) {
            if let Ok(search_result) = item.downcast::<SearchResult>() {
                execute_action(&search_result, &window_clone2, &search_entry_clone3);
            }
        }
    });

    // 7. Connect keyboard/escape to hide the window instead of closing (exiting) the application
    let window_controller = gtk::EventControllerKey::new();
    let window_clone3 = window.clone();
    window_controller.connect_key_pressed(move |_, key, _, _| {
        if key == gdk::Key::Escape {
            window_clone3.hide();
            gtk::glib::Propagation::Stop
        } else {
            gtk::glib::Propagation::Proceed
        }
    });
    window.add_controller(window_controller);

    // Connect search entry changed signal for query and debouncing
    let pending_search_id: Rc<RefCell<Option<glib::SourceId>>> = Rc::new(RefCell::new(None));
    let pending_search_clone = pending_search_id.clone();
    let list_store_clone3 = list_store.clone();
    let tracker_conn_clone = tracker_conn.clone();

    search_entry.connect_changed(move |entry| {
        if let Some(source_id) = pending_search_clone.borrow_mut().take() {
            source_id.remove();
        }

        let text = entry.text().to_string();
        if text.trim().is_empty() {
            list_store_clone3.remove_all();
            return;
        }

        let pending_search_clone2 = pending_search_clone.clone();
        let list_store_clone4 = list_store_clone3.clone();
        let tracker_conn_clone2 = tracker_conn_clone.clone();

        let source_id = glib::timeout_add_local_once(std::time::Duration::from_millis(150), move || {
            pending_search_clone2.borrow_mut().take();
            perform_search(&text, &list_store_clone4, &tracker_conn_clone2);
        });

        *pending_search_clone.borrow_mut() = Some(source_id);
    });

    // Populate mock initial instructions when first showing empty
    populate_mock_data(&list_store);

    (window, search_entry)
}

fn perform_search(query: &str, list_store: &gio::ListStore, tracker_conn: &Option<SparqlConnection>) {
    let query_lower = query.to_lowercase();
    let query_text = query.to_string();
    let list_store_clone = list_store.clone();
    let connection_opt = tracker_conn.clone();

    // Spawn an async block on the main context
    glib::MainContext::default().spawn_local(async move {
        let mut results = Vec::new();

        // 1. Search for Applications using GTK's native gio::AppInfo
        // This is synchronous but extremely fast (reads from memory cache in glib).
        let apps = gio::AppInfo::all();
        let mut app_count = 0;
        for app in apps {
            let name = app.display_name().to_string();
            let name_lower = name.to_lowercase();
            
            let desc = app.description().map(|d| d.to_string()).unwrap_or_default();
            let desc_lower = desc.to_lowercase();
            
            let exec = app.executable().to_string_lossy().to_string();
            let exec_lower = exec.to_lowercase();

            if name_lower.contains(&query_lower) || desc_lower.contains(&query_lower) || exec_lower.contains(&query_lower) {
                // Try to extract native system icon name from GIcon
                let mut icon_name = "application-x-executable".to_string();
                if let Some(icon) = app.icon() {
                    if let Some(themed) = icon.downcast_ref::<gio::ThemedIcon>() {
                        let names = themed.names();
                        if !names.is_empty() {
                            icon_name = names[0].to_string();
                        }
                    } else if let Some(gicon_str) = icon.to_string() {
                        icon_name = gicon_str.to_string();
                    }
                }

                results.push(SearchResult::new_app(&name, &format!("App: {}", exec), &icon_name, &app));
                app_count += 1;
                if app_count >= 15 {
                    break;
                }
            }
        }

        // 2. Search for Files using Tracker 3 (if available)
        if let Some(connection) = connection_opt {
            let escaped_query = query_text.replace('\'', "\\'");
            let files_query = format!(
                "SELECT ?name ?url WHERE {{ \
                    ?file a nfo:FileDataObject ; \
                          nie:url ?url ; \
                          nfo:fileName ?name . \
                    FILTER (contains(lcase(?name), lcase('{}'))) \
                 }} LIMIT 15",
                escaped_query
            );

            if let Ok(cursor) = connection.query_future(&files_query).await {
                while let Ok(has_next) = cursor.next_future().await {
                    if !has_next {
                        break;
                    }
                    let name = cursor.string(0).map(|s| s.to_string()).unwrap_or_default();
                    let url = cursor.string(1).map(|s| s.to_string()).unwrap_or_default();

                    let friendly_path = if url.starts_with("file://") {
                        url.replacen("file://", "", 1)
                    } else {
                        url.clone()
                    };

                    results.push(SearchResult::new_file(&name, &friendly_path, "text-x-generic", &url));
                }
            }
        }

        // 3. Fallback if nothing found
        if results.is_empty() {
            results.push(SearchResult::new_mock(
                "No results found",
                &format!("No matches found for '{}'", query_text),
                "dialog-information"
            ));
        }

        list_store_clone.remove_all();
        for item in results {
            list_store_clone.append(&item);
        }
    });
}

fn execute_action(result: &SearchResult, window: &ApplicationWindow, search_entry: &Entry) {
    if let Some(ref app_info) = *result.imp().app_info.borrow() {
        let context: Option<&gio::AppLaunchContext> = None;
        if let Err(e) = app_info.launch(&[], context) {
            eprintln!("Error launching application {}: {}", app_info.display_name(), e);
        }
    } else if let Some(ref file_uri) = *result.imp().file_uri.borrow() {
        if let Err(e) = gio::AppInfo::launch_default_for_uri(file_uri, None::<&gio::AppLaunchContext>) {
            eprintln!("Error opening file {}: {}", file_uri, e);
        }
    } else {
        println!("Mock action triggered for: {}", result.title());
    }

    // Hide the spotlight window and clear text for the next trigger
    window.hide();
    search_entry.set_text("");
}

fn populate_mock_data(list_store: &gio::ListStore) {
    let items = vec![
        ("Terminal", "Launch terminal emulator", "utilities-terminal"),
        ("Files", "Open file manager", "system-file-manager"),
        ("System Settings", "Configure your system", "preferences-system"),
    ];

    for (title, subtitle, icon) in items {
        let item = SearchResult::new_mock(title, subtitle, icon);
        list_store.append(&item);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_search_result_creation() {
        gtk::init().unwrap();
        let result = SearchResult::new_mock("Test Title", "Test Subtitle", "test-icon");
        assert_eq!(result.title(), "Test Title");
        assert_eq!(result.subtitle(), "Test Subtitle");
        assert_eq!(result.icon_name(), "test-icon");
    }
}
