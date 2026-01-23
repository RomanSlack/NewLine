use serde::{Deserialize, Serialize};

// Emoji constants
pub const DONE_EMOJI: &str = "✅";
pub const IN_PROGRESS_EMOJI: &str = "🟠";
pub const PENDING_MARKER: &str = "⬜";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TaskStatus {
    Pending,
    InProgress,
    Done,
    Note,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub text: String,
    pub status: TaskStatus,
    pub line_number: usize,
}

impl Task {
    pub fn from_line(line: &str, line_number: usize) -> Self {
        let trimmed = line.trim_start();

        // Check for emoji markers first
        if trimmed.starts_with(DONE_EMOJI) || trimmed.starts_with("✓") {
            Task {
                text: trimmed.chars().skip(DONE_EMOJI.chars().count()).collect::<String>().trim().to_string(),
                status: TaskStatus::Done,
                line_number,
            }
        } else if trimmed.starts_with(IN_PROGRESS_EMOJI) {
            Task {
                text: trimmed.chars().skip(IN_PROGRESS_EMOJI.chars().count()).collect::<String>().trim().to_string(),
                status: TaskStatus::InProgress,
                line_number,
            }
        } else if trimmed.starts_with(PENDING_MARKER) {
            Task {
                text: trimmed.chars().skip(PENDING_MARKER.chars().count()).collect::<String>().trim().to_string(),
                status: TaskStatus::Pending,
                line_number,
            }
        // Also support traditional markdown checkbox format
        } else if trimmed.starts_with("- [x]") || trimmed.starts_with("- [X]") {
            Task {
                text: trimmed[5..].trim().to_string(),
                status: TaskStatus::Done,
                line_number,
            }
        } else if trimmed.starts_with("- [ ]") {
            Task {
                text: trimmed[5..].trim().to_string(),
                status: TaskStatus::Pending,
                line_number,
            }
        } else if trimmed.starts_with("- ") {
            // Bullet point without checkbox - treat as pending task
            Task {
                text: trimmed[2..].trim().to_string(),
                status: TaskStatus::Pending,
                line_number,
            }
        } else if !trimmed.is_empty() && !trimmed.starts_with('#') {
            // Regular text line (not a heading) - could be a task without marker
            Task {
                text: line.to_string(),
                status: TaskStatus::Note,
                line_number,
            }
        } else {
            Task {
                text: line.to_string(),
                status: TaskStatus::Note,
                line_number,
            }
        }
    }

    pub fn to_line(&self) -> String {
        match self.status {
            TaskStatus::Done => format!("{} {}", DONE_EMOJI, self.text),
            TaskStatus::InProgress => format!("{} {}", IN_PROGRESS_EMOJI, self.text),
            TaskStatus::Pending => format!("{} {}", PENDING_MARKER, self.text),
            TaskStatus::Note => self.text.clone(),
        }
    }

    pub fn is_task(&self) -> bool {
        matches!(self.status, TaskStatus::Pending | TaskStatus::InProgress | TaskStatus::Done)
    }

    pub fn is_done(&self) -> bool {
        matches!(self.status, TaskStatus::Done)
    }
}

pub fn parse_tasks(content: &str) -> Vec<Task> {
    content
        .lines()
        .enumerate()
        .map(|(i, line)| Task::from_line(line, i))
        .collect()
}

pub fn count_tasks(content: &str) -> (usize, usize) {
    let tasks = parse_tasks(content);
    let total = tasks.iter().filter(|t| t.is_task()).count();
    let done = tasks.iter().filter(|t| t.is_done()).count();
    (done, total)
}

/// Toggle task status: Pending -> InProgress -> Done -> Pending
pub fn toggle_task_at_line(content: &str, line_number: usize) -> String {
    content
        .lines()
        .enumerate()
        .map(|(i, line)| {
            if i == line_number {
                let trimmed = line.trim_start();
                let indent = &line[..line.len() - trimmed.len()];

                // Handle emoji formats - cycle through: Pending -> InProgress -> Done -> Pending
                if trimmed.starts_with(DONE_EMOJI) {
                    // Done -> Pending
                    let text: String = trimmed.chars().skip(DONE_EMOJI.chars().count()).collect();
                    format!("{}{} {}", indent, PENDING_MARKER, text.trim())
                } else if trimmed.starts_with(IN_PROGRESS_EMOJI) {
                    // InProgress -> Done
                    let text: String = trimmed.chars().skip(IN_PROGRESS_EMOJI.chars().count()).collect();
                    format!("{}{} {}", indent, DONE_EMOJI, text.trim())
                } else if trimmed.starts_with(PENDING_MARKER) {
                    // Pending -> InProgress
                    let text: String = trimmed.chars().skip(PENDING_MARKER.chars().count()).collect();
                    format!("{}{} {}", indent, IN_PROGRESS_EMOJI, text.trim())
                // Handle traditional markdown checkbox format
                } else if trimmed.starts_with("- [x]") || trimmed.starts_with("- [X]") {
                    // Convert to pending emoji format
                    format!("{}{} {}", indent, PENDING_MARKER, trimmed[5..].trim())
                } else if trimmed.starts_with("- [ ]") {
                    // Convert to in-progress emoji format
                    format!("{}{} {}", indent, IN_PROGRESS_EMOJI, trimmed[5..].trim())
                } else if trimmed.starts_with("- ") {
                    // Bullet - convert to in-progress
                    format!("{}{} {}", indent, IN_PROGRESS_EMOJI, trimmed[2..].trim())
                } else if !trimmed.is_empty() && !trimmed.starts_with('#') {
                    // Plain text - convert to in-progress task
                    format!("{}{} {}", indent, IN_PROGRESS_EMOJI, trimmed)
                } else {
                    line.to_string()
                }
            } else {
                line.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

pub fn create_new_task_line(current_line: &str) -> String {
    let trimmed = current_line.trim_start();
    let indent = &current_line[..current_line.len() - trimmed.len()];

    // If current line looks like a task with emoji, create new task with pending marker
    if trimmed.starts_with(DONE_EMOJI) || trimmed.starts_with(IN_PROGRESS_EMOJI) || trimmed.starts_with(PENDING_MARKER) {
        format!("{}{} ", indent, PENDING_MARKER)
    } else if trimmed.starts_with("- [") {
        // Traditional checkbox format - use emoji instead
        format!("{}{} ", indent, PENDING_MARKER)
    } else if trimmed.starts_with("- ") {
        format!("{}- ", indent)
    } else {
        format!("{}{} ", indent, PENDING_MARKER)
    }
}
