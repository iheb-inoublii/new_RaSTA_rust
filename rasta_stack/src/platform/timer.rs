// Timer trait as specified in the document.
// We use this so we don't depend on Linux or OS-specific timers.

pub trait Timer {
    fn start(&mut self, duration_ms: u32);
    fn expired(&self) -> bool;
    fn stop(&mut self);
}
