use adw::prelude::*;
use gtk::gdk;
use gtk::gio;
use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;

use crate::task;

#[derive(Clone)]
pub struct ProjectSidebar {
    pub widget: gtk::Box,
    pub list_box: gtk::ListBox,
    projects_dir: Rc<RefCell<PathBuf>>,
    on_file_selected: Rc<RefCell<Option<Box<dyn Fn(PathBuf)>>>>,
    pinned: Rc<RefCell<Vec<String>>>,  // List of pinned project filenames
}

impl ProjectSidebar {
    pub fn new() -> Self {
        let projects_dir = dirs::document_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("NextLine");

        // Create projects directory if it doesn't exist
        let _ = std::fs::create_dir_all(&projects_dir);

        let widget = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .spacing(0)
            .width_request(220)
            .css_classes(["sidebar"])
            .build();

        // Header with title and new project button
        let header = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .spacing(6)
            .margin_start(12)
            .margin_end(12)
            .margin_top(12)
            .margin_bottom(6)
            .build();

        let title = gtk::Label::builder()
            .label("Projects")
            .css_classes(["title-4"])
            .hexpand(true)
            .halign(gtk::Align::Start)
            .build();

        let new_btn = gtk::Button::builder()
            .icon_name("list-add-symbolic")
            .tooltip_text("New Project")
            .css_classes(["flat", "circular"])
            .build();

        header.append(&title);
        header.append(&new_btn);

        // Scrollable list of projects
        let scrolled = gtk::ScrolledWindow::builder()
            .vexpand(true)
            .hscrollbar_policy(gtk::PolicyType::Never)
            .build();

        let list_box = gtk::ListBox::builder()
            .selection_mode(gtk::SelectionMode::Single)
            .css_classes(["navigation-sidebar"])
            .build();

        scrolled.set_child(Some(&list_box));

        widget.append(&header);
        widget.append(&scrolled);

        // Load pinned projects
        let pinned_file = projects_dir.join(".pinned");
        let pinned: Vec<String> = std::fs::read_to_string(&pinned_file)
            .map(|s| s.lines().map(|l| l.to_string()).filter(|l| !l.is_empty()).collect())
            .unwrap_or_default();

        let sidebar = Self {
            widget,
            list_box,
            projects_dir: Rc::new(RefCell::new(projects_dir)),
            on_file_selected: Rc::new(RefCell::new(None)),
            pinned: Rc::new(RefCell::new(pinned)),
        };

        // Connect new button
        let sidebar_clone = sidebar.clone();
        new_btn.connect_clicked(move |_| {
            sidebar_clone.create_new_project();
        });

        // Connect row activation
        let sidebar_clone = sidebar.clone();
        sidebar.list_box.connect_row_activated(move |_, row| {
            // ActionRow inherits from ListBoxRow, so row IS the ActionRow
            // Just get the widget_name directly from the row
            let path_str = row.widget_name().to_string();
            if !path_str.is_empty() && !path_str.starts_with("Gtk") && !path_str.starts_with("Adw") {
                let path = PathBuf::from(path_str);
                if let Some(ref callback) = *sidebar_clone.on_file_selected.borrow() {
                    callback(path);
                }
            }
        });

        // Load initial projects
        sidebar.refresh();

        sidebar
    }

    pub fn set_on_file_selected<F: Fn(PathBuf) + 'static>(&self, callback: F) {
        *self.on_file_selected.borrow_mut() = Some(Box::new(callback));
    }

    pub fn refresh(&self) {
        // Clear existing rows
        while let Some(child) = self.list_box.first_child() {
            self.list_box.remove(&child);
        }

        // Read project files
        let projects_dir = self.projects_dir.borrow().clone();
        let pinned = self.pinned.borrow().clone();

        if let Ok(entries) = std::fs::read_dir(&projects_dir) {
            let mut files: Vec<PathBuf> = entries
                .filter_map(|e| e.ok())
                .map(|e| e.path())
                .filter(|p| {
                    p.is_file()
                        && p.extension()
                            .map(|e| e == "txt" || e == "md" || e == "todo")
                            .unwrap_or(false)
                })
                .collect();

            // Sort: pinned first, then by modification time (newest first)
            files.sort_by(|a, b| {
                let a_name = a.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
                let b_name = b.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
                let a_pinned = pinned.contains(&a_name);
                let b_pinned = pinned.contains(&b_name);

                match (a_pinned, b_pinned) {
                    (true, false) => std::cmp::Ordering::Less,
                    (false, true) => std::cmp::Ordering::Greater,
                    _ => {
                        // Same pin status, sort by modification time
                        let a_time = a.metadata().and_then(|m| m.modified()).ok();
                        let b_time = b.metadata().and_then(|m| m.modified()).ok();
                        b_time.cmp(&a_time)
                    }
                }
            });

            for path in files {
                let is_pinned = path.file_name()
                    .map(|n| pinned.contains(&n.to_string_lossy().to_string()))
                    .unwrap_or(false);
                self.add_project_row(&path, is_pinned);
            }
        }
    }

    fn add_project_row(&self, path: &PathBuf, is_pinned: bool) {
        let row = adw::ActionRow::builder()
            .activatable(true)
            .build();

        // Store path as widget name
        row.set_widget_name(&path.to_string_lossy());

        // Set title from filename
        let filename = path
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "Untitled".to_string());
        row.set_title(&filename);

        // Read file and count tasks for subtitle
        if let Ok(content) = std::fs::read_to_string(path) {
            let (done, total) = task::count_tasks(&content);
            if total > 0 {
                row.set_subtitle(&format!("{}/{} tasks", done, total));

                // Add progress indicator
                let progress = gtk::LevelBar::builder()
                    .min_value(0.0)
                    .max_value(total as f64)
                    .value(done as f64)
                    .valign(gtk::Align::Center)
                    .width_request(60)
                    .build();
                progress.add_css_class("caption");
                row.add_suffix(&progress);
            }
        }

        // Add icon
        row.add_prefix(&gtk::Image::from_icon_name("text-x-generic-symbolic"));

        // Add pin icon on the right if pinned
        if is_pinned {
            let pin_icon = gtk::Image::builder()
                .icon_name("view-pin-symbolic")
                .css_classes(["dim-label"])
                .build();
            row.add_suffix(&pin_icon);
        }

        // Create context menu for right-click
        let menu = gio::Menu::new();
        menu.append(Some("Rename"), Some("sidebar.rename"));
        menu.append(Some("Pin"), Some("sidebar.pin"));
        menu.append(Some("Delete"), Some("sidebar.delete"));
        menu.append(Some("Show in Folder"), Some("sidebar.show-in-folder"));

        let popover = gtk::PopoverMenu::from_model(Some(&menu));
        popover.set_parent(&row);
        popover.set_has_arrow(false);

        // Setup actions for the row
        let action_group = gio::SimpleActionGroup::new();

        // Rename action
        let path_clone = path.clone();
        let sidebar = self.clone();
        let rename_action = gio::SimpleAction::new("rename", None);
        rename_action.connect_activate(move |_, _| {
            sidebar.show_rename_dialog(&path_clone);
        });
        action_group.add_action(&rename_action);

        // Pin action
        let path_clone = path.clone();
        let sidebar = self.clone();
        let pin_action = gio::SimpleAction::new("pin", None);
        pin_action.connect_activate(move |_, _| {
            sidebar.toggle_pin(&path_clone);
        });
        action_group.add_action(&pin_action);

        // Delete action
        let path_clone = path.clone();
        let sidebar = self.clone();
        let delete_action = gio::SimpleAction::new("delete", None);
        delete_action.connect_activate(move |_, _| {
            sidebar.show_delete_confirmation(&path_clone);
        });
        action_group.add_action(&delete_action);

        // Show in folder action
        let path_clone = path.clone();
        let show_action = gio::SimpleAction::new("show-in-folder", None);
        show_action.connect_activate(move |_, _| {
            if let Some(parent) = path_clone.parent() {
                let _ = open::that(parent);
            }
        });
        action_group.add_action(&show_action);

        row.insert_action_group("sidebar", Some(&action_group));

        // Right-click gesture
        let gesture = gtk::GestureClick::new();
        gesture.set_button(gdk::BUTTON_SECONDARY);
        let popover_clone = popover.clone();
        gesture.connect_pressed(move |gesture, _, x, y| {
            gesture.set_state(gtk::EventSequenceState::Claimed);
            popover_clone.set_pointing_to(Some(&gdk::Rectangle::new(x as i32, y as i32, 1, 1)));
            popover_clone.popup();
        });
        row.add_controller(gesture);

        self.list_box.append(&row);
    }

    fn show_rename_dialog(&self, path: &PathBuf) {
        let Some(root) = self.widget.root() else { return };
        let Some(window) = root.downcast_ref::<gtk::Window>() else { return };

        let dialog = gtk::Dialog::builder()
            .title("Rename Project")
            .transient_for(window)
            .modal(true)
            .build();

        dialog.add_button("Cancel", gtk::ResponseType::Cancel);
        dialog.add_button("Rename", gtk::ResponseType::Accept);

        let content = dialog.content_area();
        content.set_margin_top(12);
        content.set_margin_bottom(12);
        content.set_margin_start(12);
        content.set_margin_end(12);
        content.set_spacing(12);

        let label = gtk::Label::builder()
            .label("Enter a new name:")
            .halign(gtk::Align::Start)
            .build();

        let current_name = path
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_default();

        let entry = gtk::Entry::builder()
            .text(&current_name)
            .activates_default(true)
            .hexpand(true)
            .build();

        content.append(&label);
        content.append(&entry);

        dialog.set_default_response(gtk::ResponseType::Accept);

        let path_clone = path.clone();
        let sidebar = self.clone();

        dialog.connect_response(move |dialog, response| {
            if response == gtk::ResponseType::Accept {
                let new_name = entry.text().to_string();
                if !new_name.is_empty() && new_name != current_name {
                    let extension = path_clone.extension()
                        .map(|e| e.to_string_lossy().to_string())
                        .unwrap_or_else(|| "txt".to_string());
                    let new_path = path_clone.parent()
                        .map(|p| p.join(format!("{}.{}", new_name, extension)))
                        .unwrap_or_else(|| PathBuf::from(format!("{}.{}", new_name, extension)));

                    if std::fs::rename(&path_clone, &new_path).is_ok() {
                        sidebar.refresh();
                    }
                }
            }
            dialog.close();
        });

        dialog.present();
    }

    fn show_delete_confirmation(&self, path: &PathBuf) {
        let Some(root) = self.widget.root() else { return };
        let Some(window) = root.downcast_ref::<gtk::Window>() else { return };

        let filename = path
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "this project".to_string());

        let dialog = gtk::Dialog::builder()
            .title("Delete Project")
            .transient_for(window)
            .modal(true)
            .build();

        dialog.add_button("Cancel", gtk::ResponseType::Cancel);
        let delete_btn = dialog.add_button("Delete", gtk::ResponseType::Accept);
        delete_btn.add_css_class("destructive-action");

        let content = dialog.content_area();
        content.set_margin_top(12);
        content.set_margin_bottom(12);
        content.set_margin_start(12);
        content.set_margin_end(12);
        content.set_spacing(12);

        let icon = gtk::Image::builder()
            .icon_name("dialog-warning-symbolic")
            .pixel_size(48)
            .build();

        let label = gtk::Label::builder()
            .label(&format!("Are you sure you want to delete \"{}\"?\n\nThis action cannot be undone.", filename))
            .wrap(true)
            .max_width_chars(40)
            .build();

        content.append(&icon);
        content.append(&label);

        let path_clone = path.clone();
        let sidebar = self.clone();

        dialog.connect_response(move |dialog, response| {
            if response == gtk::ResponseType::Accept {
                if std::fs::remove_file(&path_clone).is_ok() {
                    sidebar.refresh();
                }
            }
            dialog.close();
        });

        dialog.present();
    }

    pub fn create_new_project(&self) {
        // Get the window for the dialog
        let Some(root) = self.widget.root() else { return };
        let Some(window) = root.downcast_ref::<gtk::Window>() else { return };

        // Create a simple dialog for new project name
        let dialog = gtk::Dialog::builder()
            .title("New Project")
            .transient_for(window)
            .modal(true)
            .build();

        dialog.add_button("Cancel", gtk::ResponseType::Cancel);
        dialog.add_button("Create", gtk::ResponseType::Accept);

        let content = dialog.content_area();
        content.set_margin_top(12);
        content.set_margin_bottom(12);
        content.set_margin_start(12);
        content.set_margin_end(12);
        content.set_spacing(12);

        let label = gtk::Label::builder()
            .label("Enter a name for your new project:")
            .halign(gtk::Align::Start)
            .build();

        let entry = gtk::Entry::builder()
            .placeholder_text("Project name")
            .activates_default(true)
            .hexpand(true)
            .build();

        content.append(&label);
        content.append(&entry);

        dialog.set_default_response(gtk::ResponseType::Accept);

        let projects_dir = self.projects_dir.borrow().clone();
        let sidebar = self.clone();

        dialog.connect_response(move |dialog, response| {
            if response == gtk::ResponseType::Accept {
                let name = entry.text().to_string();
                if !name.is_empty() {
                    let path = projects_dir.join(format!("{}.txt", name));

                    // Create initial content with emoji markers
                    let initial = format!(
                        "# {}\n\n⬜ First task\n⬜ Second task\n",
                        name
                    );

                    if std::fs::write(&path, initial).is_ok() {
                        sidebar.refresh();

                        // Auto-select the new project
                        if let Some(ref callback) = *sidebar.on_file_selected.borrow() {
                            callback(path);
                        }
                    }
                }
            }
            dialog.close();
        });

        dialog.present();
    }

    pub fn projects_dir(&self) -> PathBuf {
        self.projects_dir.borrow().clone()
    }

    fn toggle_pin(&self, path: &PathBuf) {
        let filename = path.file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();

        if filename.is_empty() {
            return;
        }

        {
            let mut pinned = self.pinned.borrow_mut();
            if let Some(pos) = pinned.iter().position(|p| p == &filename) {
                pinned.remove(pos);
            } else {
                pinned.push(filename);
            }
        }

        self.save_pinned();
        self.refresh();
    }

    fn save_pinned(&self) {
        let projects_dir = self.projects_dir.borrow().clone();
        let pinned_file = projects_dir.join(".pinned");
        let pinned = self.pinned.borrow();
        let content = pinned.join("\n");
        let _ = std::fs::write(pinned_file, content);
    }
}
