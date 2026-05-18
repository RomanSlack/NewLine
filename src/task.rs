use serde::{Deserialize, Serialize};

// Task markers
pub const DONE_EMOJI: &str = "✅";
pub const IN_PROGRESS_EMOJI: &str = "🟠";
pub const PENDING_MARKER: &str = "-";
pub const COMPLETED_SECTION_HEADER: &str = "--- Completed ---";
// Non-breaking space to keep marker and first word together when wrapping
pub const NBSP: &str = "\u{00A0}";

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
            TaskStatus::Done => format!("{}{}{}", DONE_EMOJI, NBSP, self.text),
            TaskStatus::InProgress => format!("{}{}{}", IN_PROGRESS_EMOJI, NBSP, self.text),
            TaskStatus::Pending => format!("{}{}{}", PENDING_MARKER, NBSP, self.text),
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
                // Use NBSP (non-breaking space) to keep marker with first word when wrapping
                if trimmed.starts_with(DONE_EMOJI) {
                    // Done -> Pending
                    let text: String = trimmed.chars().skip(DONE_EMOJI.chars().count()).collect();
                    format!("{}{}{}{}", indent, PENDING_MARKER, NBSP, text.trim())
                } else if trimmed.starts_with(IN_PROGRESS_EMOJI) {
                    // InProgress -> Done
                    let text: String = trimmed.chars().skip(IN_PROGRESS_EMOJI.chars().count()).collect();
                    format!("{}{}{}{}", indent, DONE_EMOJI, NBSP, text.trim())
                } else if trimmed.starts_with(PENDING_MARKER) {
                    // Pending -> InProgress
                    let text: String = trimmed.chars().skip(PENDING_MARKER.chars().count()).collect();
                    format!("{}{}{}{}", indent, IN_PROGRESS_EMOJI, NBSP, text.trim())
                // Handle traditional markdown checkbox format
                } else if trimmed.starts_with("- [x]") || trimmed.starts_with("- [X]") {
                    // Convert to pending emoji format
                    format!("{}{}{}{}", indent, PENDING_MARKER, NBSP, trimmed[5..].trim())
                } else if trimmed.starts_with("- [ ]") {
                    // Convert to in-progress emoji format
                    format!("{}{}{}{}", indent, IN_PROGRESS_EMOJI, NBSP, trimmed[5..].trim())
                } else if trimmed.starts_with("- ") {
                    // Bullet - convert to in-progress
                    format!("{}{}{}{}", indent, IN_PROGRESS_EMOJI, NBSP, trimmed[2..].trim())
                } else if !trimmed.is_empty() && !trimmed.starts_with('#') {
                    // Plain text - convert to in-progress task
                    format!("{}{}{}{}", indent, IN_PROGRESS_EMOJI, NBSP, trimmed)
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

/// Toggle a single line's task status without rebuilding the whole document
pub fn toggle_single_line(line: &str) -> String {
    let trimmed = line.trim_start();
    let indent = &line[..line.len() - trimmed.len()];

    if trimmed.starts_with(DONE_EMOJI) {
        let text: String = trimmed.chars().skip(DONE_EMOJI.chars().count()).collect();
        format!("{}{}{}{}", indent, PENDING_MARKER, NBSP, text.trim())
    } else if trimmed.starts_with(IN_PROGRESS_EMOJI) {
        let text: String = trimmed.chars().skip(IN_PROGRESS_EMOJI.chars().count()).collect();
        format!("{}{}{}{}", indent, DONE_EMOJI, NBSP, text.trim())
    } else if trimmed.starts_with(PENDING_MARKER) {
        let text: String = trimmed.chars().skip(PENDING_MARKER.chars().count()).collect();
        format!("{}{}{}{}", indent, IN_PROGRESS_EMOJI, NBSP, text.trim())
    } else if trimmed.starts_with("- [x]") || trimmed.starts_with("- [X]") {
        format!("{}{}{}{}", indent, PENDING_MARKER, NBSP, trimmed[5..].trim())
    } else if trimmed.starts_with("- [ ]") {
        format!("{}{}{}{}", indent, IN_PROGRESS_EMOJI, NBSP, trimmed[5..].trim())
    } else if trimmed.starts_with("- ") {
        format!("{}{}{}{}", indent, IN_PROGRESS_EMOJI, NBSP, trimmed[2..].trim())
    } else if !trimmed.is_empty() && !trimmed.starts_with('#') {
        format!("{}{}{}{}", indent, IN_PROGRESS_EMOJI, NBSP, trimmed)
    } else {
        line.to_string()
    }
}

pub fn create_new_task_line(current_line: &str) -> String {
    let trimmed = current_line.trim_start();
    let indent = &current_line[..current_line.len() - trimmed.len()];

    // If current line looks like a task with emoji, create new task with pending marker
    // Use NBSP to keep marker with first word when wrapping
    if trimmed.starts_with(DONE_EMOJI) || trimmed.starts_with(IN_PROGRESS_EMOJI) || trimmed.starts_with(PENDING_MARKER) {
        format!("{}{}{}", indent, PENDING_MARKER, NBSP)
    } else if trimmed.starts_with("- [") {
        // Traditional checkbox format - use emoji instead
        format!("{}{}{}", indent, PENDING_MARKER, NBSP)
    } else if trimmed.starts_with("- ") {
        format!("{}-{}", indent, NBSP)
    } else {
        format!("{}{}{}", indent, PENDING_MARKER, NBSP)
    }
}

/// Collapse completed tasks to the bottom with no blank lines between them
/// Keep pending and in-progress tasks at top with their original formatting
pub fn collapse_completed(content: &str) -> String {
    let mut top_section: Vec<String> = Vec::new();
    let mut completed_tasks: Vec<String> = Vec::new();
    let mut current_block: Vec<String> = Vec::new();
    let mut block_has_active_task = false;

    for line in content.lines() {
        let trimmed = line.trim();

        // Drop any existing generated section header so collapse stays idempotent.
        if trimmed == COMPLETED_SECTION_HEADER {
            continue;
        }

        // Check if this line is a completed task
        let is_completed = trimmed.starts_with(DONE_EMOJI)
            || trimmed.starts_with("- [x]")
            || trimmed.starts_with("- [X]");

        // Check if this line is an active task (pending or in-progress)
        let is_active_task = trimmed.starts_with(IN_PROGRESS_EMOJI)
            || trimmed.starts_with(PENDING_MARKER)
            || trimmed.starts_with("- [ ]")
            || (trimmed.starts_with("- ") && !is_completed);

        if is_completed {
            // If we have a block with active tasks, flush it to top section
            if block_has_active_task {
                top_section.extend(current_block.drain(..));
            } else if !current_block.is_empty() {
                // Block only has non-task content, add to top
                top_section.extend(current_block.drain(..));
            }
            // Add completed task (just the task line, no surrounding whitespace)
            completed_tasks.push(line.to_string());
            block_has_active_task = false;
        } else if is_active_task {
            // This is an active task
            current_block.push(line.to_string());
            block_has_active_task = true;
        } else if trimmed.is_empty() {
            // Blank line - add to current block
            current_block.push(line.to_string());
        } else {
            // Regular text (headings, notes, etc)
            current_block.push(line.to_string());
        }
    }

    // Flush remaining block
    if !current_block.is_empty() {
        top_section.extend(current_block);
    }

    while top_section
        .last()
        .map(|line| line.trim().is_empty())
        .unwrap_or(false)
    {
        top_section.pop();
    }

    // Build result: top section, then a separator, then collapsed completed tasks
    let mut result = top_section.join("\n");

    if !completed_tasks.is_empty() {
        // Add separator if we have content above
        if !result.trim().is_empty() {
            result.push_str(&format!("\n\n{}\n", COMPLETED_SECTION_HEADER));
        }
        // Add completed tasks with no blank lines between them
        result.push_str(&completed_tasks.join("\n"));
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn collapse_completed_moves_done_tasks_below_header() {
        let input = format!(
            "# Project\n\n{}{}Task one\n{}{}Task two\n{}{}Task three",
            PENDING_MARKER, NBSP, DONE_EMOJI, NBSP, IN_PROGRESS_EMOJI, NBSP
        );

        let output = collapse_completed(&input);

        let expected = format!(
            "# Project\n\n{}{}Task one\n{}{}Task three\n\n{}\n{}{}Task two",
            PENDING_MARKER,
            NBSP,
            IN_PROGRESS_EMOJI,
            NBSP,
            COMPLETED_SECTION_HEADER,
            DONE_EMOJI,
            NBSP
        );

        assert_eq!(output, expected);
    }

    #[test]
    fn collapse_completed_is_idempotent() {
        let input = format!(
            "# Project\n\n{}{}Task one\n\n{}\n{}{}Task two",
            PENDING_MARKER, NBSP, COMPLETED_SECTION_HEADER, DONE_EMOJI, NBSP
        );

        let once = collapse_completed(&input);
        let twice = collapse_completed(&once);

        assert_eq!(once, twice);
        assert_eq!(once.matches(COMPLETED_SECTION_HEADER).count(), 1);
    }
}
