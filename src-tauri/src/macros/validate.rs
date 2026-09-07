//! Import/save validation. A malformed file must produce a readable error,
//! never a panic and never an unbounded macro (§23, §40, §41).

use serde::Serialize;

use super::model::{
    Action, ActionKind, Macro, LoopMode, MAX_ACTIONS, MAX_INTERVAL_MS, MAX_NESTING, MAX_TEXT_LEN,
    MIN_INTERVAL_MS,
};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ValidationError {
    pub field: String,
    pub message: String,
}

impl ValidationError {
    fn new(field: impl Into<String>, message: impl Into<String>) -> Self {
        Self { field: field.into(), message: message.into() }
    }
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.field, self.message)
    }
}

impl std::error::Error for ValidationError {}

/// Validates and normalises in place. Out-of-range numbers are clamped rather
/// than rejected; only structurally impossible input is an error.
pub fn normalize_macro(value: &mut Macro) -> Result<(), ValidationError> {
    if value.id.trim().is_empty() {
        return Err(ValidationError::new("id", "Macro id is empty"));
    }
    if value.name.trim().is_empty() {
        value.name = "Untitled macro".into();
    }
    if value.name.chars().count() > 80 {
        value.name = value.name.chars().take(80).collect();
    }

    let total = count_actions(&value.actions);
    if total > MAX_ACTIONS {
        return Err(ValidationError::new(
            "actions",
            format!("Macro has {total} actions, the limit is {MAX_ACTIONS}"),
        ));
    }

    normalize_actions(&mut value.actions, 0)?;

    match value.loop_config.mode {
        LoopMode::Count => {
            value.loop_config.count = value.loop_config.count.clamp(1, 1_000_000);
        }
        LoopMode::Duration => {
            value.loop_config.duration_ms = value.loop_config.duration_ms.clamp(100, 86_400_000);
        }
        LoopMode::None | LoopMode::Infinite => {}
    }

    // 0 is legal here and means "no gap"; the engine keeps its own floor.
    value.loop_config.interval_ms = value.loop_config.interval_ms.min(MAX_INTERVAL_MS);

    if value.randomization.enabled {
        value.randomization.jitter_ms = value.randomization.jitter_ms.min(5_000);
    }

    Ok(())
}

fn count_actions(actions: &[Action]) -> usize {
    actions
        .iter()
        .map(|action| match &action.kind {
            ActionKind::Repeat { actions, .. } => 1 + count_actions(actions),
            _ => 1,
        })
        .sum()
}

fn normalize_actions(actions: &mut [Action], depth: usize) -> Result<(), ValidationError> {
    if depth > MAX_NESTING {
        return Err(ValidationError::new(
            "actions",
            format!("Repeat blocks are nested deeper than {MAX_NESTING} levels"),
        ));
    }

    for action in actions.iter_mut() {
        if action.id.trim().is_empty() {
            return Err(ValidationError::new("action.id", "Action id is empty"));
        }
        action.delay_before_ms = action.delay_before_ms.min(MAX_INTERVAL_MS);
        action.delay_after_ms = action.delay_after_ms.min(MAX_INTERVAL_MS);

        match &mut action.kind {
            ActionKind::MouseClick { count, interval_ms, duration_ms, .. } => {
                *count = (*count).clamp(1, 100_000);
                *interval_ms = (*interval_ms).clamp(MIN_INTERVAL_MS, MAX_INTERVAL_MS);
                *duration_ms = (*duration_ms).min(MAX_INTERVAL_MS);
            }
            ActionKind::MouseMove { x, y, duration_ms, .. } => {
                *x = (*x).clamp(-100_000, 100_000);
                *y = (*y).clamp(-100_000, 100_000);
                *duration_ms = (*duration_ms).min(60_000);
            }
            ActionKind::MouseScroll { amount, interval_ms, .. } => {
                *amount = (*amount).clamp(1, 1_000);
                *interval_ms = (*interval_ms).clamp(MIN_INTERVAL_MS, MAX_INTERVAL_MS);
            }
            ActionKind::Key { code, modifiers, duration_ms, .. } => {
                if code.trim().is_empty() {
                    return Err(ValidationError::new("action.code", "Key action has no key"));
                }
                modifiers.sort_by_key(|m| format!("{m:?}"));
                modifiers.dedup();
                *duration_ms = (*duration_ms).min(MAX_INTERVAL_MS);
            }
            ActionKind::Text { value, per_char_delay_ms } => {
                if value.chars().count() > MAX_TEXT_LEN {
                    *value = value.chars().take(MAX_TEXT_LEN).collect();
                }
                *per_char_delay_ms = (*per_char_delay_ms).min(5_000);
            }
            ActionKind::Delay { duration_ms } => {
                *duration_ms = (*duration_ms).min(MAX_INTERVAL_MS);
            }
            ActionKind::Repeat { times, actions } => {
                *times = (*times).clamp(1, 100_000);
                normalize_actions(actions, depth + 1)?;
            }
        }
    }

    Ok(())
}
