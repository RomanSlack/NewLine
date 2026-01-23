use gtk::prelude::*;
use sourceview::prelude::*;
use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;

use crate::task;

pub type DocumentHandle = Rc<RefCell<Document>>;

pub struct Document {
    pub path: Option<PathBuf>,
    pub view: sourceview::View,
    pub buffer: sourceview::Buffer,
    modified: bool,
}

impl Document {
    pub fn new(path: Option<PathBuf>) -> DocumentHandle {
        let buffer = sourceview::Buffer::builder()
            .highlight_syntax(false)
            .highlight_matching_brackets(false)
            .build();

        let view = sourceview::View::with_buffer(&buffer);

        // Configure view for a clean, minimal text editor look (like GNOME Text Editor)
        view.set_monospace(false);  // Use system font, not monospace
        view.set_show_line_numbers(false);  // No line numbers
        view.set_highlight_current_line(false);  // No line highlight
        view.set_vexpand(true);
        view.set_hexpand(false);  // Don't expand horizontally - allow wrapping
        view.set_wrap_mode(gtk::WrapMode::WordChar);  // Wrap at words, then chars if needed
        view.set_left_margin(16);
        view.set_right_margin(16);
        view.set_top_margin(12);
        view.set_bottom_margin(12);

        // Enable unlimited undo/redo
        buffer.set_max_undo_levels(u32::MAX);

        let doc = Rc::new(RefCell::new(Self {
            path,
            view,
            buffer,
            modified: false,
        }));

        // Track modifications (use try_borrow_mut to avoid panic during load/toggle)
        let doc_weak = Rc::downgrade(&doc);
        doc.borrow().buffer.connect_changed(move |_| {
            if let Some(doc) = doc_weak.upgrade() {
                if let Ok(mut doc_mut) = doc.try_borrow_mut() {
                    doc_mut.modified = true;
                }
            }
        });

        // Load content if path provided
        let should_load = {
            let doc_ref = doc.borrow();
            doc_ref.path.as_ref().map(|p| p.exists()).unwrap_or(false)
        };
        if should_load {
            let _ = doc.borrow_mut().load_from_disk();
        }

        doc
    }

    pub fn widget(&self) -> gtk::ScrolledWindow {
        gtk::ScrolledWindow::builder()
            .vexpand(true)
            .hexpand(true)
            .hscrollbar_policy(gtk::PolicyType::Never)
            .child(&self.view)
            .build()
    }

    pub fn title(&self) -> String {
        let base = self
            .path
            .as_ref()
            .and_then(|p| p.file_name().map(|s| s.to_string_lossy().to_string()))
            .unwrap_or_else(|| "Untitled".to_string());

        if self.modified {
            format!("{}*", base)
        } else {
            base
        }
    }

    pub fn is_modified(&self) -> bool {
        self.modified
    }

    pub fn load_from_disk(&mut self) -> anyhow::Result<()> {
        let Some(path) = self.path.clone() else {
            return Ok(());
        };
        let text = std::fs::read_to_string(&path)?;
        self.buffer.set_text(&text);
        self.modified = false;
        Ok(())
    }

    pub fn save_to_disk(&mut self) -> anyhow::Result<()> {
        let Some(path) = self.path.clone() else {
            anyhow::bail!("No path set. Use Save As.");
        };

        // Create parent directories if they don't exist
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let start = self.buffer.start_iter();
        let end = self.buffer.end_iter();
        let text = self.buffer.text(&start, &end, true).to_string();
        std::fs::write(&path, text)?;
        self.modified = false;
        Ok(())
    }

    pub fn set_path(&mut self, path: PathBuf) {
        self.path = Some(path);
    }

    pub fn get_content(&self) -> String {
        let start = self.buffer.start_iter();
        let end = self.buffer.end_iter();
        self.buffer.text(&start, &end, true).to_string()
    }

    pub fn get_task_stats(&self) -> (usize, usize) {
        task::count_tasks(&self.get_content())
    }

    pub fn undo(&self) {
        if self.buffer.can_undo() {
            self.buffer.undo();
        }
    }

    pub fn redo(&self) {
        if self.buffer.can_redo() {
            self.buffer.redo();
        }
    }

    pub fn get_current_line_number(&self) -> usize {
        let mark = self.buffer.get_insert();
        let iter = self.buffer.iter_at_mark(&mark);
        iter.line() as usize
    }

    pub fn toggle_current_task(&self) {
        let content = self.get_content();
        let line = self.get_current_line_number();
        let new_content = task::toggle_task_at_line(&content, line);

        // Save cursor position
        let mark = self.buffer.get_insert();
        let cursor = self.buffer.iter_at_mark(&mark);
        let offset = cursor.offset();

        self.buffer.set_text(&new_content);

        // Restore cursor position
        let mut iter = self.buffer.start_iter();
        iter.set_offset(offset.min(self.buffer.end_iter().offset()));
        self.buffer.place_cursor(&iter);
    }

    pub fn insert_new_task(&self) {
        let mark = self.buffer.get_insert();
        let iter = self.buffer.iter_at_mark(&mark);
        let line_num = iter.line();

        // Get current line text
        let line_start = self.buffer.iter_at_line(line_num).unwrap();
        let mut line_end = line_start.clone();
        line_end.forward_to_line_end();
        let current_line = self.buffer.text(&line_start, &line_end, true).to_string();

        // Create new task line
        let new_task = task::create_new_task_line(&current_line);

        // Move to end of current line and insert newline + task prefix
        self.buffer.place_cursor(&line_end);
        self.buffer.insert_at_cursor(&format!("\n{}", new_task));
    }

    pub fn collapse_completed(&self) {
        let content = self.get_content();
        let collapsed = task::collapse_completed(&content);
        self.buffer.set_text(&collapsed);
        // Move cursor to start
        let start = self.buffer.start_iter();
        self.buffer.place_cursor(&start);
    }
}
