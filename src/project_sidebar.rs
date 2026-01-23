use adw::prelude::*;
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

        let sidebar = Self {
            widget,
            list_box,
            projects_dir: Rc::new(RefCell::new(projects_dir)),
            on_file_selected: Rc::new(RefCell::new(None)),
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

            // Sort by modification time (newest first)
            files.sort_by(|a, b| {
                let a_time = a.metadata().and_then(|m| m.modified()).ok();
                let b_time = b.metadata().and_then(|m| m.modified()).ok();
                b_time.cmp(&a_time)
            });

            for path in files {
                self.add_project_row(&path);
            }
        }
    }

    fn add_project_row(&self, path: &PathBuf) {
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

        self.list_box.append(&row);
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
}
