use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UpdateMode {
    AutoReset, // Check -> Download -> Restart (Immediate)
    AutoLoad,  // Check -> Download -> Wait (Apply Next Run)
    Prompt,    // Prompt -> Download -> Prompt (User choice)
    Ignore,    // Nothing
}

impl Default for UpdateMode {
    fn default() -> Self {
        Self::Prompt
    }
}

impl UpdateMode {
    pub fn label(&self) -> &'static str {
        match self {
            Self::AutoReset => "Auto-Reset",
            Self::AutoLoad => "Auto-Load",
            Self::Prompt => "Prompt",
            Self::Ignore => "Ignore",
        }
    }
}