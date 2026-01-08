use std::sync::mpsc::Sender;
use eframe::egui;

pub struct Logger {
    tx: Sender<String>,
}

impl Logger {
    pub fn new(tx: Sender<String>) -> Self {
        Self { tx }
    }

    /// Sends a standard info message
    pub fn info(&self, msg: impl Into<String>) {
        let _ = self.tx.send(msg.into());
    }

    /// Sends an error message
    pub fn error(&self, msg: impl Into<String>) {
        let _ = self.tx.send(format!("Error: {}", msg.into()));
    }
    
    /// Sends a success message
    pub fn success(&self, msg: impl Into<String>) {
        let _ = self.tx.send(format!("Success: {}", msg.into()));
    }
}

pub fn resolve_status_color(msg: &str) -> egui::Color32 {
    if msg.contains("Error") || msg.contains("Aborted") {
        return egui::Color32::RED;
    }
    if msg.contains("Success") {
        return egui::Color32::GREEN;
    }
    egui::Color32::LIGHT_BLUE
}

pub fn resolve_log_color(line: &str) -> egui::Color32 {
    if line.contains("was found!") || line.contains("Success") {
        return egui::Color32::GREEN;
    }
    if line.contains("Error") || line.contains("Aborted") {
        return egui::Color32::RED;
    }
    egui::Color32::WHITE
}