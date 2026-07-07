use gtk4 as gtk;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::{
    Application, ApplicationWindow, Box, Entry, Orientation, ListView, ScrolledWindow,
    CssProvider, style_context_add_provider_for_display, Label, PolicyType,
    SignalListItemFactory, SingleSelection
};
use gtk::gio;
use gtk::gdk;
use gdk::Display;
use std::rc::Rc;
use std::cell::RefCell;

// Custom GObject for holding search results
mod imp {
    use gtk4 as gtk;
    use gtk::glib;
    use gtk::subclass::prelude::*;
    use std::cell::RefCell;

    #[derive(Default)]
    pub struct SearchResult {
        pub title: RefCell<String>,
        pub subtitle: RefCell<String>,
        pub icon: RefCell<String>,
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
    pub fn new(title: &str, subtitle: &str, icon: &str) -> Self {
        let obj: Self = glib::Object::builder().build();
        *obj.imp().title.borrow_mut() = title.to_string();
        *obj.imp().subtitle.borrow_mut() = subtitle.to_string();
        *obj.imp().icon.borrow_mut() = icon.to_string();
        obj
    }

    pub fn title(&self) -> String {
        self.imp().title.borrow().clone()
    }

    pub fn subtitle(&self) -> String {
        self.imp().subtitle.borrow().clone()
    }

    pub fn icon(&self) -> String {
        self.imp().icon.borrow().clone()
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

listview {
    background: transparent;
    border: none;
}

/* List item row styling */
listview  row {
    padding: 8px 12px;
    border-radius: 8px;
    margin: 2px 0;
    color: #e0e0e0;
}

listview  row:hover {
    background-color: rgba(255, 255, 255, 0.05);
}

listview  row:selected {
    background-color: #3584e4;
    color: #ffffff;
}

.row-title {
    font-size: 15px;
    font-weight: bold;
}

.row-subtitle {
    font-size: 12px;
    color: #909090;
}

listview  row:selected .row-subtitle {
    color: #d0d0d0;
}
";

fn main() {
    let app = Application::builder()
        .application_id("com.sammy.LinuxSpotlight")
        .flags(gio::ApplicationFlags::HANDLES_COMMAND_LINE)
        .build();

    // Application state: holds references to the window and search entry
    let state: Rc<RefCell<Option<(ApplicationWindow, Entry)>>> = Rc::new(RefCell::new(None));

    let state_clone = state.clone();
    app.connect_activate(move |app| {
        if state_clone.borrow().is_none() {
            let (window, search_entry) = build_ui(app);
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

fn build_ui(app: &Application) -> (ApplicationWindow, Entry) {
    // 1. Create Window
    let window = ApplicationWindow::builder()
        .application(app)
        .title("Linux Spotlight")
        .default_width(650)
        .default_height(120) // starts small, expands
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

    // Single selection model
    let selection_model = SingleSelection::new(Some(list_store.clone()));

    // ListView factory
    let factory = SignalListItemFactory::new();

    factory.connect_setup(move |_, list_item| {
        let row_box = Box::builder()
            .orientation(Orientation::Vertical)
            .spacing(2)
            .build();

        let title_label = Label::builder()
            .halign(gtk::Align::Start)
            .build();
        title_label.add_css_class("row-title");

        let subtitle_label = Label::builder()
            .halign(gtk::Align::Start)
            .build();
        subtitle_label.add_css_class("row-subtitle");

        row_box.append(&title_label);
        row_box.append(&subtitle_label);

        list_item.set_child(Some(&row_box));
    });

    factory.connect_bind(move |_, list_item| {
        let item = list_item.item().unwrap();
        let search_result = item.downcast_ref::<SearchResult>().unwrap();

        let row_box = list_item.child().unwrap().downcast::<Box>().unwrap();
        let title_label = row_box.first_child().unwrap().downcast::<Label>().unwrap();
        let subtitle_label = title_label.next_sibling().unwrap().downcast::<Label>().unwrap();

        title_label.set_text(&search_result.title());
        subtitle_label.set_text(&search_result.subtitle());
    });

    let list_view = ListView::new(Some(selection_model.clone()), Some(factory));
    
    scrolled_window.set_child(Some(&list_view));
    main_box.append(&scrolled_window);

    window.set_child(Some(&main_box));

    // 4. Connect keyboard navigation redirect on the search entry
    let selection_model_clone = selection_model.clone();
    let list_store_clone = list_store.clone();
    let entry_controller = gtk::EventControllerKey::new();
    entry_controller.connect_key_pressed(move |_, key, _, _| {
        let current_selected = selection_model_clone.selected();
        let total_items = list_store_clone.n_items();

        match key {
            gdk::Key::Down => {
                if total_items > 0 {
                    if current_selected == gtk::INVALID_LIST_POSITION {
                        selection_model_clone.set_selected(0);
                    } else if current_selected < total_items - 1 {
                        selection_model_clone.set_selected(current_selected + 1);
                    }
                }
                gtk::glib::Propagation::Stop
            }
            gdk::Key::Up => {
                if total_items > 0 {
                    if current_selected != gtk::INVALID_LIST_POSITION && current_selected > 0 {
                        selection_model_clone.set_selected(current_selected - 1);
                    }
                }
                gtk::glib::Propagation::Stop
            }
            gdk::Key::Return => {
                if current_selected != gtk::INVALID_LIST_POSITION {
                    if let Some(item) = list_store_clone.item(current_selected) {
                        if let Ok(search_result) = item.downcast::<SearchResult>() {
                            println!("Activated item: {}", search_result.title());
                        }
                    }
                }
                gtk::glib::Propagation::Proceed
            }
            _ => gtk::glib::Propagation::Proceed
        }
    });
    search_entry.add_controller(entry_controller);

    // 5. Connect keyboard/escape to hide the window instead of closing (exiting) the application
    let window_controller = gtk::EventControllerKey::new();
    let window_clone = window.clone();
    window_controller.connect_key_pressed(move |_, key, _, _| {
        if key == gdk::Key::Escape {
            window_clone.hide();
            gtk::glib::Propagation::Stop
        } else {
            gtk::glib::Propagation::Proceed
        }
    });
    window.add_controller(window_controller);

    // Populate mock data
    populate_mock_data(&list_store);

    (window, search_entry)
}

fn populate_mock_data(list_store: &gio::ListStore) {
    let items = vec![
        ("Terminal", "Launch terminal emulator", "utilities-terminal"),
        ("Files", "Open file manager", "system-file-manager"),
        ("System Settings", "Configure your system", "preferences-system"),
    ];

    for (title, subtitle, icon) in items {
        let item = SearchResult::new(title, subtitle, icon);
        list_store.append(&item);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_search_result_creation() {
        gtk::init().unwrap();
        let result = SearchResult::new("Test Title", "Test Subtitle", "test-icon");
        assert_eq!(result.title(), "Test Title");
        assert_eq!(result.subtitle(), "Test Subtitle");
        assert_eq!(result.icon(), "test-icon");
    }
}
