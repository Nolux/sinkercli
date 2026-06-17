pub mod detect;
pub mod pulse;

use anyhow::Result;

#[derive(Debug, Clone)]
pub struct Source {
    pub name: String,
    pub description: String,
    pub is_monitor: bool,
}

#[derive(Debug, Clone)]
pub struct Sink {
    pub name: String,
    pub description: String,
    pub is_virtual: bool,
}

#[derive(Debug, Clone)]
pub struct Loopback {
    pub module_id: u32,
    pub source_name: String,
    pub sink_name: String,
}

#[derive(Debug, Clone)]
pub struct SinkInput {
    pub id: u32,
    pub app_name: String,
    pub current_sink_name: String,
}

pub trait AudioBackend: Send + Sync {
    fn sources(&self) -> Result<Vec<Source>>;
    fn sinks(&self) -> Result<Vec<Sink>>;
    fn loopbacks(&self) -> Result<Vec<Loopback>>;
    fn sink_inputs(&self) -> Result<Vec<SinkInput>>;
    fn create_loopback(&self, source: &Source, sink: &Sink) -> Result<Loopback>;
    fn create_virtual_sink(&self, name: &str) -> Result<Sink>;
    fn remove_loopback(&self, lb: &Loopback) -> Result<()>;
    fn remove_virtual_sink(&self, sink: &Sink) -> Result<()>;
    fn move_sink_input(&self, input: &SinkInput, sink: &Sink) -> Result<()>;
    /// Wrap a sink's monitor as a proper virtual microphone input via module-virtual-source.
    fn create_virtual_source(&self, name: &str, monitor_source: &str) -> Result<Source>;
}
