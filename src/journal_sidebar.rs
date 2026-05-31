use adw::prelude::*;
use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;

use chrono::NaiveDate;

use crate::journal;
use crate::task;

#[derive(Clone)]
pub struct JournalSidebar {
    pub widget: gtk::Box,
    list_box: gtk::ListBox,
    calendar: gtk::Calendar,
    selected_path: Rc<RefCell<Option<PathBuf>>>,
    on_file_selected: Rc<RefCell<Option<Box<dyn Fn(PathBuf)>>>>,
}

impl JournalSidebar {
    pub fn new() -> Self {
        let widget = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .spacing(0)
            .width_request(220)
            .css_classes(["sidebar"])
            .build();

        // Header with title and "Today" button
        let header = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .spacing(6)
            .margin_start(12)
            .margin_end(12)
            .margin_top(12)
            .margin_bottom(6)
            .build();

        let title = gtk::Label::builder()
            .label("Daily")
            .css_classes(["title-4"])
            .hexpand(true)
            .halign(gtk::Align::Start)
            .build();

        let today_btn = gtk::Button::builder()
            .icon_name("x-office-calendar-symbolic")
            .tooltip_text("Go to today")
            .css_classes(["flat", "circular"])
            .cursor(&gtk::gdk::Cursor::from_name("pointer", None).unwrap())
            .build();

        header.append(&title);
        header.append(&today_btn);

        // Calendar for browsing past days
        let calendar = gtk::Calendar::builder()
            .margin_start(6)
            .margin_end(6)
            .margin_bottom(6)
            .build();

        // Scrollable list of existing entries (newest first)
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
        widget.append(&calendar);
        widget.append(&scrolled);

        let sidebar = Self {
            widget,
            list_box,
            calendar,
            selected_path: Rc::new(RefCell::new(None)),
            on_file_selected: Rc::new(RefCell::new(None)),
        };

        // "Today" button opens (creating if needed) today's entry
        let s = sidebar.clone();
        today_btn.connect_clicked(move |_| {
            s.open_today();
        });

        // Selecting a day opens (creating if needed) that day's entry
        let s = sidebar.clone();
        sidebar.calendar.connect_day_selected(move |cal| {
            let dt = cal.date();
            if let Some(date) =
                journal::date_from_ymd(dt.year(), dt.month() as u32, dt.day_of_month() as u32)
            {
                s.open_date(date);
            }
        });

        // Re-mark entry days whenever the displayed month/year changes
        let s = sidebar.clone();
        sidebar
            .calendar
            .connect_notify_local(Some("month"), move |_, _| s.update_marks());
        let s = sidebar.clone();
        sidebar
            .calendar
            .connect_notify_local(Some("year"), move |_, _| s.update_marks());

        // Row activation opens the entry
        let s = sidebar.clone();
        sidebar.list_box.connect_row_activated(move |_, row| {
            let path_str = row.widget_name().to_string();
            if !path_str.is_empty() && !path_str.starts_with("Gtk") && !path_str.starts_with("Adw")
            {
                let path = PathBuf::from(&path_str);
                if let Some(date) = journal::parse_date(&path) {
                    s.open_date(date);
                }
            }
        });

        sidebar.refresh();

        sidebar
    }

    pub fn set_on_file_selected<F: Fn(PathBuf) + 'static>(&self, callback: F) {
        *self.on_file_selected.borrow_mut() = Some(Box::new(callback));
    }

    /// Open (creating from template if missing) the entry for a given day.
    pub fn open_date(&self, date: NaiveDate) {
        let path = journal::ensure(date);
        *self.selected_path.borrow_mut() = Some(path.clone());
        self.refresh();
        if let Some(ref callback) = *self.on_file_selected.borrow() {
            callback(path);
        }
    }

    pub fn open_today(&self) {
        self.open_date(journal::today());
    }

    /// List existing entries (newest first) and refresh calendar marks.
    pub fn refresh(&self) {
        // Clear existing rows
        while let Some(child) = self.list_box.first_child() {
            self.list_box.remove(&child);
        }

        // Collect entry dates
        let mut dates: Vec<NaiveDate> = Vec::new();
        if let Ok(entries) = std::fs::read_dir(journal::journal_dir()) {
            for entry in entries.filter_map(|e| e.ok()) {
                let path = entry.path();
                if path.is_file() {
                    if let Some(date) = journal::parse_date(&path) {
                        dates.push(date);
                    }
                }
            }
        }

        // Newest first
        dates.sort();
        dates.reverse();

        for date in &dates {
            self.add_entry_row(*date);
        }

        self.update_marks();

        // Restore selection
        if let Some(ref selected) = *self.selected_path.borrow() {
            let selected_name = selected.to_string_lossy().to_string();
            let mut index = 0;
            while let Some(row) = self.list_box.row_at_index(index) {
                if row.widget_name().to_string() == selected_name {
                    self.list_box.select_row(Some(&row));
                    break;
                }
                index += 1;
            }
        }
    }

    fn add_entry_row(&self, date: NaiveDate) {
        let path = journal::path_for(date);

        let row = adw::ActionRow::builder()
            .activatable(true)
            .cursor(&gtk::gdk::Cursor::from_name("pointer", None).unwrap())
            .build();

        row.set_widget_name(&path.to_string_lossy());
        row.set_title(&journal::display_title(date));

        // Task counts (same pattern as the projects sidebar)
        if let Ok(content) = std::fs::read_to_string(&path) {
            let (done, total) = task::count_tasks(&content);
            if total > 0 {
                row.set_subtitle(&format!("{}/{} tasks", done, total));

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

        row.add_prefix(&gtk::Image::from_icon_name("view-paged-symbolic"));

        self.list_box.append(&row);
    }

    /// Mark days in the calendar's currently displayed month that have an entry.
    fn update_marks(&self) {
        self.calendar.clear_marks();

        let year = self.calendar.year();
        // Calendar's `month` property is 0-based (0 = January).
        let month = (self.calendar.month() + 1) as u32;

        if let Ok(entries) = std::fs::read_dir(journal::journal_dir()) {
            for entry in entries.filter_map(|e| e.ok()) {
                if let Some(date) = journal::parse_date(&entry.path()) {
                    use chrono::Datelike;
                    if date.year() == year && date.month() == month {
                        self.calendar.mark_day(journal::day_of_month(date));
                    }
                }
            }
        }
    }
}
