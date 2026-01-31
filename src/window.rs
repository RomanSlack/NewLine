use adw::prelude::*;
use gtk::{gio, glib};
use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;

use crate::document::{Document, DocumentHandle};
use crate::project_sidebar::ProjectSidebar;
use crate::sync::state::AppConfig;

pub struct MainWindow {
    pub window: adw::ApplicationWindow,
    sidebar: ProjectSidebar,
    content_stack: gtk::Stack,
    documents: Rc<RefCell<Vec<DocumentHandle>>>,
    current_doc: Rc<RefCell<Option<DocumentHandle>>>,
    progress_label: gtk::Label,
    title_label: gtk::Label,
    sync_button: gtk::Button,
    sync_spinner: gtk::Spinner,
    toast_overlay: adw::ToastOverlay,
}

impl MainWindow {
    pub fn new(app: &adw::Application) -> Rc<Self> {
        let window = adw::ApplicationWindow::builder()
            .application(app)
            .title("NextLine")
            .default_width(1000)
            .default_height(700)
            .build();

        // Main horizontal layout with resizable sidebar using Paned
        let paned = gtk::Paned::builder()
            .orientation(gtk::Orientation::Horizontal)
            .shrink_start_child(false)
            .shrink_end_child(false)
            .position(220)  // Initial sidebar width
            .build();

        // Create sidebar
        let sidebar = ProjectSidebar::new();
        paned.set_start_child(Some(&sidebar.widget));

        // Content area with header and editor
        let content_box = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .hexpand(true)
            .build();

        // Header bar
        let header = adw::HeaderBar::builder()
            .build();

        // Title in center
        let title_label = gtk::Label::builder()
            .label("NextLine")
            .css_classes(["title"])
            .build();
        header.set_title_widget(Some(&title_label));

        // Progress label on the right
        let progress_label = gtk::Label::builder()
            .label("")
            .css_classes(["caption"])
            .margin_start(12)
            .margin_end(12)
            .build();
        header.pack_end(&progress_label);

        // Collapse button - moves completed tasks to bottom
        let collapse_btn = gtk::Button::builder()
            .icon_name("view-sort-descending-symbolic")
            .tooltip_text("Collapse completed tasks to bottom")
            .css_classes(["flat"])
            .action_name("win.collapse")
            .cursor(&gtk::gdk::Cursor::from_name("pointer", None).unwrap())
            .build();
        header.pack_end(&collapse_btn);

        // Sync button with spinner overlay
        let sync_button = gtk::Button::builder()
            .icon_name("emblem-synchronizing-symbolic")
            .tooltip_text("Sync with cloud")
            .css_classes(["flat"])
            .action_name("win.sync")
            .cursor(&gtk::gdk::Cursor::from_name("pointer", None).unwrap())
            .build();

        let sync_spinner = gtk::Spinner::builder()
            .visible(false)
            .build();

        // Create overlay for sync button with spinner
        let sync_overlay = gtk::Overlay::new();
        sync_overlay.set_child(Some(&sync_button));
        sync_overlay.add_overlay(&sync_spinner);
        header.pack_end(&sync_overlay);

        // Menu button
        let menu_btn = gtk::MenuButton::builder()
            .icon_name("open-menu-symbolic")
            .tooltip_text("Menu")
            .cursor(&gtk::gdk::Cursor::from_name("pointer", None).unwrap())
            .build();

        let menu = gio::Menu::new();
        menu.append(Some("New Task (Enter)"), Some("win.new-task"));
        menu.append(Some("Toggle Done (Ctrl+D)"), Some("win.toggle-task"));
        menu.append(Some("Collapse Completed"), Some("win.collapse"));
        menu.append(Some("Save (Ctrl+S)"), Some("win.save"));
        menu.append(Some("Undo (Ctrl+Z)"), Some("win.undo"));
        menu.append(Some("Redo (Ctrl+Shift+Z)"), Some("win.redo"));

        let sync_section = gio::Menu::new();
        sync_section.append(Some("Sync Now"), Some("win.sync"));
        sync_section.append(Some("Configure Sync..."), Some("win.configure-sync"));
        menu.append_section(None, &sync_section);

        let section = gio::Menu::new();
        section.append(Some("Open Projects Folder"), Some("win.open-folder"));
        section.append(Some("Keyboard Shortcuts"), Some("win.shortcuts"));
        section.append(Some("About"), Some("win.about"));
        menu.append_section(None, &section);

        menu_btn.set_menu_model(Some(&menu));
        header.pack_end(&menu_btn);

        content_box.append(&header);

        // Content stack for empty state and editor
        let content_stack = gtk::Stack::builder()
            .vexpand(true)
            .hexpand(true)
            .build();

        // Empty state
        let empty_state = adw::StatusPage::builder()
            .icon_name("checkbox-checked-symbolic")
            .title("Welcome to NextLine")
            .description("Select a project from the sidebar or create a new one")
            .vexpand(true)
            .build();

        let create_btn = gtk::Button::builder()
            .label("Create New Project")
            .css_classes(["suggested-action", "pill"])
            .halign(gtk::Align::Center)
            .cursor(&gtk::gdk::Cursor::from_name("pointer", None).unwrap())
            .build();
        empty_state.set_child(Some(&create_btn));

        content_stack.add_named(&empty_state, Some("empty"));

        content_box.append(&content_stack);
        paned.set_end_child(Some(&content_box));

        // Wrap everything in a toast overlay
        let toast_overlay = adw::ToastOverlay::new();
        toast_overlay.set_child(Some(&paned));

        // Set the toast overlay as the window content
        window.set_content(Some(&toast_overlay));

        let main = Rc::new(Self {
            window,
            sidebar,
            content_stack,
            documents: Rc::new(RefCell::new(Vec::new())),
            current_doc: Rc::new(RefCell::new(None)),
            progress_label,
            title_label,
            sync_button,
            sync_spinner,
            toast_overlay,
        });

        // Setup connections
        main.setup_actions();
        main.setup_sidebar_callbacks();

        // Connect create button
        let main_weak = Rc::downgrade(&main);
        create_btn.connect_clicked(move |_| {
            if let Some(main) = main_weak.upgrade() {
                main.sidebar.create_new_project();
            }
        });

        main
    }

    fn setup_actions(self: &Rc<Self>) {
        let window = &self.window;

        // Save action
        let main = Rc::downgrade(self);
        let save_action = gio::SimpleAction::new("save", None);
        save_action.connect_activate(move |_, _| {
            if let Some(main) = main.upgrade() {
                main.save_current();
            }
        });
        window.add_action(&save_action);

        // Undo action
        let main = Rc::downgrade(self);
        let undo_action = gio::SimpleAction::new("undo", None);
        undo_action.connect_activate(move |_, _| {
            if let Some(main) = main.upgrade() {
                if let Some(ref doc) = *main.current_doc.borrow() {
                    doc.borrow().undo();
                }
            }
        });
        window.add_action(&undo_action);

        // Redo action
        let main = Rc::downgrade(self);
        let redo_action = gio::SimpleAction::new("redo", None);
        redo_action.connect_activate(move |_, _| {
            if let Some(main) = main.upgrade() {
                if let Some(ref doc) = *main.current_doc.borrow() {
                    doc.borrow().redo();
                }
            }
        });
        window.add_action(&redo_action);

        // Toggle task action
        let main = Rc::downgrade(self);
        let toggle_action = gio::SimpleAction::new("toggle-task", None);
        toggle_action.connect_activate(move |_, _| {
            if let Some(main) = main.upgrade() {
                if let Some(ref doc) = *main.current_doc.borrow() {
                    doc.borrow().toggle_current_task();
                    main.update_progress();
                }
            }
        });
        window.add_action(&toggle_action);

        // New task action
        let main = Rc::downgrade(self);
        let new_task_action = gio::SimpleAction::new("new-task", None);
        new_task_action.connect_activate(move |_, _| {
            if let Some(main) = main.upgrade() {
                if let Some(ref doc) = *main.current_doc.borrow() {
                    doc.borrow().insert_new_task();
                    main.update_progress();
                }
            }
        });
        window.add_action(&new_task_action);

        // Collapse action
        let main = Rc::downgrade(self);
        let collapse_action = gio::SimpleAction::new("collapse", None);
        collapse_action.connect_activate(move |_, _| {
            if let Some(main) = main.upgrade() {
                if let Some(ref doc) = *main.current_doc.borrow() {
                    doc.borrow().collapse_completed();
                    main.update_progress();
                }
            }
        });
        window.add_action(&collapse_action);

        // Open folder action
        let main = Rc::downgrade(self);
        let open_folder_action = gio::SimpleAction::new("open-folder", None);
        open_folder_action.connect_activate(move |_, _| {
            if let Some(main) = main.upgrade() {
                let folder = main.sidebar.projects_dir();
                let _ = open::that(folder);
            }
        });
        window.add_action(&open_folder_action);

        // About action
        let main = Rc::downgrade(self);
        let about_action = gio::SimpleAction::new("about", None);
        about_action.connect_activate(move |_, _| {
            if let Some(main) = main.upgrade() {
                let about = gtk::AboutDialog::builder()
                    .transient_for(&main.window)
                    .modal(true)
                    .program_name("NextLine")
                    .logo_icon_name("checkbox-checked-symbolic")
                    .version("0.1.0")
                    .comments("A simple, GNOME-style task manager")
                    .license_type(gtk::License::Gpl30)
                    .authors(["Roman"])
                    .build();
                about.present();
            }
        });
        window.add_action(&about_action);

        // Shortcuts dialog action
        let main = Rc::downgrade(self);
        let shortcuts_action = gio::SimpleAction::new("shortcuts", None);
        shortcuts_action.connect_activate(move |_, _| {
            if let Some(main) = main.upgrade() {
                main.show_shortcuts_dialog();
            }
        });
        window.add_action(&shortcuts_action);

        // Sync action
        let main = Rc::downgrade(self);
        let sync_action = gio::SimpleAction::new("sync", None);
        sync_action.connect_activate(move |_, _| {
            if let Some(main) = main.upgrade() {
                main.perform_sync();
            }
        });
        window.add_action(&sync_action);

        // Refresh sidebar action (used internally after sync)
        let main = Rc::downgrade(self);
        let refresh_sidebar_action = gio::SimpleAction::new("refresh-sidebar", None);
        refresh_sidebar_action.connect_activate(move |_, _| {
            if let Some(main) = main.upgrade() {
                main.sidebar.refresh();
            }
        });
        window.add_action(&refresh_sidebar_action);

        // Configure sync action
        let main = Rc::downgrade(self);
        let configure_sync_action = gio::SimpleAction::new("configure-sync", None);
        configure_sync_action.connect_activate(move |_, _| {
            if let Some(main) = main.upgrade() {
                main.show_sync_config_dialog();
            }
        });
        window.add_action(&configure_sync_action);

        // Keyboard shortcuts
        let app = self.window.application().unwrap();
        app.set_accels_for_action("win.save", &["<Ctrl>s"]);
        app.set_accels_for_action("win.undo", &["<Ctrl>z"]);
        app.set_accels_for_action("win.redo", &["<Ctrl><Shift>z"]);
        app.set_accels_for_action("win.toggle-task", &["<Ctrl>d"]);
        app.set_accels_for_action("win.new-task", &["<Ctrl>Return"]);
    }

    fn setup_sidebar_callbacks(self: &Rc<Self>) {
        let main = Rc::downgrade(self);
        self.sidebar.set_on_file_selected(move |path| {
            if let Some(main) = main.upgrade() {
                main.open_document(path);
            }
        });

        let main = Rc::downgrade(self);
        self.sidebar.set_on_file_deleted(move |deleted_path| {
            if let Some(main) = main.upgrade() {
                main.handle_file_deleted(&deleted_path);
            }
        });
    }

    fn handle_file_deleted(&self, deleted_path: &PathBuf) {
        let page_name = deleted_path.to_string_lossy().to_string();

        // Check if deleted file is currently open
        let is_current = {
            if let Some(ref doc) = *self.current_doc.borrow() {
                doc.borrow().path.as_ref() == Some(deleted_path)
            } else {
                false
            }
        };

        if is_current {
            // Clear current document
            *self.current_doc.borrow_mut() = None;

            // Show empty state
            self.content_stack.set_visible_child_name("empty");

            // Reset title and progress
            self.title_label.set_label("NextLine");
            self.progress_label.set_label("");
        }

        // Remove the widget from the stack
        if let Some(child) = self.content_stack.child_by_name(&page_name) {
            self.content_stack.remove(&child);
        }

        // Remove from documents list
        self.documents.borrow_mut().retain(|doc| {
            doc.borrow().path.as_ref() != Some(deleted_path)
        });
    }

    pub fn open_document(&self, path: PathBuf) {
        let page_name = path.to_string_lossy().to_string();

        // Check if document is already open
        let existing_doc = self.documents.borrow().iter()
            .find(|doc| doc.borrow().path.as_ref() == Some(&path))
            .cloned();

        if let Some(doc) = existing_doc {
            // Document already open, just switch to it
            self.content_stack.set_visible_child_name(&page_name);
            *self.current_doc.borrow_mut() = Some(doc);
            self.update_title(&path);
            self.update_progress();
            return;
        }

        // Create new document
        let doc = Document::new(Some(path.clone()));

        // Get the editor widget
        let editor = doc.borrow().widget();

        // Add to stack
        self.content_stack.add_named(&editor, Some(&page_name));
        self.content_stack.set_visible_child_name(&page_name);

        // Setup key controller for the view
        let doc_weak = Rc::downgrade(&doc);
        let key_controller = gtk::EventControllerKey::new();

        let main_progress = self.progress_label.clone();
        let current_doc = self.current_doc.clone();

        key_controller.connect_key_pressed(move |_, key, _, modifier| {
            // Handle Enter key to auto-create new task line
            if key == gtk::gdk::Key::Return && !modifier.contains(gtk::gdk::ModifierType::CONTROL_MASK) {
                if let Some(doc) = doc_weak.upgrade() {
                    // Check if we're at the end of a task line
                    let doc_ref = doc.borrow();
                    let content = doc_ref.get_content();
                    let line_num = doc_ref.get_current_line_number();

                    if let Some(line) = content.lines().nth(line_num) {
                        let trimmed = line.trim_start();
                        if trimmed.starts_with("- [") || trimmed.starts_with("- ") {
                            // Will be handled by insert_new_task
                            drop(doc_ref);
                            doc.borrow().insert_new_task();

                            // Update progress
                            if let Some(ref current) = *current_doc.borrow() {
                                let (done, total) = current.borrow().get_task_stats();
                                if total > 0 {
                                    main_progress.set_label(&format!("{}/{}", done, total));
                                } else {
                                    main_progress.set_label("");
                                }
                            }

                            return glib::Propagation::Stop;
                        }
                    }
                }
            }
            glib::Propagation::Proceed
        });

        doc.borrow().view.add_controller(key_controller);

        // Update current doc and title
        *self.current_doc.borrow_mut() = Some(doc.clone());
        self.documents.borrow_mut().push(doc);

        self.update_title(&path);
        self.update_progress();
    }

    fn update_title(&self, path: &PathBuf) {
        let name = path
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "Untitled".to_string());
        self.title_label.set_label(&name);
    }

    fn update_progress(&self) {
        if let Some(ref doc) = *self.current_doc.borrow() {
            let (done, total) = doc.borrow().get_task_stats();
            if total > 0 {
                self.progress_label.set_label(&format!("{}/{}", done, total));
            } else {
                self.progress_label.set_label("");
            }
        }
    }

    fn save_current(&self) {
        if let Some(ref doc) = *self.current_doc.borrow() {
            let _ = doc.borrow_mut().save_to_disk();
            self.sidebar.refresh();
        }
    }

    fn show_shortcuts_dialog(&self) {
        let dialog = gtk::Dialog::builder()
            .title("Keyboard Shortcuts")
            .transient_for(&self.window)
            .modal(true)
            .build();

        dialog.add_button("Close", gtk::ResponseType::Close);

        let content = dialog.content_area();
        content.set_margin_top(12);
        content.set_margin_bottom(12);
        content.set_margin_start(24);
        content.set_margin_end(24);

        let grid = gtk::Grid::builder()
            .row_spacing(8)
            .column_spacing(24)
            .build();

        let shortcuts = [
            ("Ctrl+S", "Save"),
            ("Ctrl+Z", "Undo"),
            ("Ctrl+Shift+Z", "Redo"),
            ("Ctrl+D", "Cycle: ⬜ → 🟠 → ✅"),
            ("Ctrl+Enter", "New task"),
            ("Enter", "New task (on task line)"),
        ];

        for (i, (key, action)) in shortcuts.iter().enumerate() {
            let key_label = gtk::Label::builder()
                .label(*key)
                .css_classes(["dim-label"])
                .halign(gtk::Align::End)
                .build();
            let action_label = gtk::Label::builder()
                .label(*action)
                .halign(gtk::Align::Start)
                .build();
            grid.attach(&key_label, 0, i as i32, 1, 1);
            grid.attach(&action_label, 1, i as i32, 1, 1);
        }

        content.append(&grid);

        dialog.connect_response(|dialog, _| {
            dialog.close();
        });

        dialog.present();
    }

    pub fn present(&self) {
        self.window.present();
    }

    fn perform_sync(&self) {
        // Save current document first
        self.save_current();

        // Check if sync is configured
        let config = AppConfig::load();
        if !config.sync.is_configured() {
            self.show_toast("Sync not configured. Use menu to configure.");
            return;
        }

        // Show spinner
        self.sync_button.set_visible(false);
        self.sync_spinner.set_visible(true);
        self.sync_spinner.start();

        let projects_dir = self.sidebar.projects_dir();
        let sync_button = self.sync_button.clone();
        let sync_spinner = self.sync_spinner.clone();
        let toast_overlay = self.toast_overlay.clone();

        let window = self.window.clone();

        // Use a channel to send result from background thread to main thread
        let (tx, rx) = std::sync::mpsc::channel::<Result<String, String>>();

        // Run sync in background thread
        std::thread::spawn(move || {
            let result: Result<String, String> = match crate::sync::SyncManager::new(&projects_dir) {
                Ok(mut manager) => match manager.sync() {
                    Ok(sync_result) => Ok(sync_result.summary()),
                    Err(e) => Err(e.to_string()),
                },
                Err(e) => Err(e.to_string()),
            };
            let _ = tx.send(result);
        });

        // Poll for result using glib timeout (runs on main thread)
        glib::timeout_add_local(std::time::Duration::from_millis(100), move || {
            match rx.try_recv() {
                Ok(result) => {
                    sync_spinner.stop();
                    sync_spinner.set_visible(false);
                    sync_button.set_visible(true);

                    match result {
                        Ok(summary) => {
                            let toast = adw::Toast::new(&summary);
                            toast.set_timeout(3);
                            toast_overlay.add_toast(toast);
                        }
                        Err(e) => {
                            let toast = adw::Toast::new(&format!("Sync failed: {}", e));
                            toast.set_timeout(5);
                            toast_overlay.add_toast(toast);
                        }
                    }

                    // Trigger sidebar refresh via action
                    if let Some(action) = window.lookup_action("refresh-sidebar") {
                        action.activate(None);
                    }

                    glib::ControlFlow::Break
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => glib::ControlFlow::Continue,
                Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    // Thread ended without sending - shouldn't happen
                    sync_spinner.stop();
                    sync_spinner.set_visible(false);
                    sync_button.set_visible(true);
                    glib::ControlFlow::Break
                }
            }
        });
    }

    fn show_toast(&self, message: &str) {
        let toast = adw::Toast::new(message);
        toast.set_timeout(3);
        self.toast_overlay.add_toast(toast);
    }

    fn show_sync_config_dialog(&self) {
        let config = AppConfig::load();

        let dialog = gtk::Dialog::builder()
            .title("Configure Sync")
            .transient_for(&self.window)
            .modal(true)
            .build();

        dialog.add_button("Cancel", gtk::ResponseType::Cancel);
        dialog.add_button("Save", gtk::ResponseType::Accept);

        let content = dialog.content_area();
        content.set_margin_top(12);
        content.set_margin_bottom(12);
        content.set_margin_start(24);
        content.set_margin_end(24);
        content.set_spacing(12);

        // Enabled checkbox
        let enabled_check = gtk::CheckButton::builder()
            .label("Enable cloud sync")
            .active(config.sync.enabled)
            .build();
        content.append(&enabled_check);

        // URL entry
        let url_box = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .spacing(4)
            .build();
        let url_label = gtk::Label::builder()
            .label("Sync URL")
            .halign(gtk::Align::Start)
            .build();
        let url_entry = gtk::Entry::builder()
            .placeholder_text("https://nextline-sync.your.workers.dev")
            .text(&config.sync.url)
            .hexpand(true)
            .build();
        url_box.append(&url_label);
        url_box.append(&url_entry);
        content.append(&url_box);

        // API key entry
        let key_box = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .spacing(4)
            .build();
        let key_label = gtk::Label::builder()
            .label("API Key")
            .halign(gtk::Align::Start)
            .build();
        let key_entry = gtk::PasswordEntry::builder()
            .placeholder_text("Your secret API key")
            .show_peek_icon(true)
            .hexpand(true)
            .build();
        key_entry.set_text(&config.sync.api_key);
        key_box.append(&key_label);
        key_box.append(&key_entry);
        content.append(&key_box);

        // Help text
        let help_label = gtk::Label::builder()
            .label("Set up a Cloudflare Worker with R2 storage.\nSee documentation for deployment instructions.")
            .css_classes(["dim-label", "caption"])
            .halign(gtk::Align::Start)
            .wrap(true)
            .build();
        content.append(&help_label);

        dialog.connect_response(move |dialog, response| {
            if response == gtk::ResponseType::Accept {
                let new_config = AppConfig {
                    sync: crate::sync::state::SyncConfig {
                        enabled: enabled_check.is_active(),
                        url: url_entry.text().to_string(),
                        api_key: key_entry.text().to_string(),
                    },
                };
                if let Err(e) = new_config.save() {
                    eprintln!("Failed to save config: {}", e);
                }
            }
            dialog.close();
        });

        dialog.present();
    }
}
