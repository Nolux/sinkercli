use crate::audio::{AudioBackend, Loopback, Sink, SinkInput, Source};
use crate::audio::detect::AudioSystem;
use crate::config::{Config, LoopbackEntry, Preset};
use anyhow::Result;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Panel {
    Sources,
    Sinks,
    Applications,
    Loopbacks,
}

impl Panel {
    pub fn next(self) -> Self {
        match self {
            Panel::Sources => Panel::Sinks,
            Panel::Sinks => Panel::Applications,
            Panel::Applications => Panel::Loopbacks,
            Panel::Loopbacks => Panel::Sources,
        }
    }

    pub fn prev(self) -> Self {
        match self {
            Panel::Sources => Panel::Loopbacks,
            Panel::Sinks => Panel::Sources,
            Panel::Applications => Panel::Sinks,
            Panel::Loopbacks => Panel::Applications,
        }
    }
}

#[derive(Debug, Clone)]
pub enum Dialog {
    None,
    /// Step 1: pick the source. Step 2: pick the sink (source already chosen).
    NewLoopbackPickSource,
    NewLoopbackPickSink { source: Source },
    NewVirtualSink { input: String },
    /// Moving an app stream: user has selected an input and now picks the target sink.
    MoveSinkInput { input: SinkInput },
    /// Listening to a sink: route its monitor to a chosen output sink.
    ListenToSink { sink: Sink },
    /// Wrapping a sink's monitor as a virtual microphone input.
    CreateVirtualSource { sink: Sink, name: String },
    Presets { selected: usize },
    SavePreset { input: String },
    Error(String),
}


pub struct App {
    pub audio: Box<dyn AudioBackend>,
    pub audio_system: AudioSystem,

    pub sources: Vec<Source>,
    pub sinks: Vec<Sink>,
    pub loopbacks: Vec<Loopback>,
    pub sink_inputs: Vec<SinkInput>,

    pub focused: Panel,
    pub source_sel: usize,
    pub sink_sel: usize,
    pub loopback_sel: usize,
    pub sink_input_sel: usize,

    pub dialog: Dialog,
    pub config: Config,

    pub status_msg: Option<String>,
}

impl App {
    pub fn new(audio: Box<dyn AudioBackend>, audio_system: AudioSystem) -> Self {
        App {
            audio,
            audio_system,
            sources: Vec::new(),
            sinks: Vec::new(),
            loopbacks: Vec::new(),
            sink_inputs: Vec::new(),
            focused: Panel::Sources,
            source_sel: 0,
            sink_sel: 0,
            loopback_sel: 0,
            sink_input_sel: 0,
            dialog: Dialog::None,
            config: Config::load().unwrap_or_default(),
            status_msg: None,
        }
    }

    pub fn refresh(&mut self) {
        match self.audio.sources() {
            Ok(s) => {
                self.sources = s;
                self.source_sel = self.source_sel.min(self.sources.len().saturating_sub(1));
            }
            Err(e) => self.status_msg = Some(format!("sources error: {}", e)),
        }
        match self.audio.sinks() {
            Ok(s) => {
                self.sinks = s;
                self.sink_sel = self.sink_sel.min(self.sinks.len().saturating_sub(1));
            }
            Err(e) => self.status_msg = Some(format!("sinks error: {}", e)),
        }
        match self.audio.loopbacks() {
            Ok(l) => {
                self.loopbacks = l;
                self.loopback_sel = self.loopback_sel.min(self.loopbacks.len().saturating_sub(1));
            }
            Err(e) => self.status_msg = Some(format!("loopbacks error: {}", e)),
        }
        match self.audio.sink_inputs() {
            Ok(i) => {
                self.sink_inputs = i;
                self.sink_input_sel =
                    self.sink_input_sel.min(self.sink_inputs.len().saturating_sub(1));
            }
            Err(e) => self.status_msg = Some(format!("sink inputs error: {}", e)),
        }
    }

    pub fn selected_source(&self) -> Option<&Source> {
        self.sources.get(self.source_sel)
    }

    pub fn selected_sink(&self) -> Option<&Sink> {
        self.sinks.get(self.sink_sel)
    }

    pub fn selected_loopback(&self) -> Option<&Loopback> {
        self.loopbacks.get(self.loopback_sel)
    }

    pub fn selected_sink_input(&self) -> Option<&SinkInput> {
        self.sink_inputs.get(self.sink_input_sel)
    }

    // ---- key actions ----

    pub fn move_up(&mut self) {
        match self.focused {
            Panel::Sources => {
                if self.source_sel > 0 {
                    self.source_sel -= 1;
                }
            }
            Panel::Sinks => {
                if self.sink_sel > 0 {
                    self.sink_sel -= 1;
                }
            }
            Panel::Loopbacks => {
                if self.loopback_sel > 0 {
                    self.loopback_sel -= 1;
                }
            }
            Panel::Applications => {
                if self.sink_input_sel > 0 {
                    self.sink_input_sel -= 1;
                }
            }
        }
    }

    pub fn move_down(&mut self) {
        match self.focused {
            Panel::Sources => {
                if self.source_sel + 1 < self.sources.len() {
                    self.source_sel += 1;
                }
            }
            Panel::Sinks => {
                if self.sink_sel + 1 < self.sinks.len() {
                    self.sink_sel += 1;
                }
            }
            Panel::Loopbacks => {
                if self.loopback_sel + 1 < self.loopbacks.len() {
                    self.loopback_sel += 1;
                }
            }
            Panel::Applications => {
                if self.sink_input_sel + 1 < self.sink_inputs.len() {
                    self.sink_input_sel += 1;
                }
            }
        }
    }

    pub fn delete_selected(&mut self) {
        match self.focused {
            Panel::Loopbacks => {
                if let Some(lb) = self.selected_loopback().cloned() {
                    if let Err(e) = self.audio.remove_loopback(&lb) {
                        self.status_msg = Some(format!("Error: {}", e));
                    } else {
                        self.status_msg = Some(format!(
                            "Removed loopback {} → {}",
                            lb.source_name, lb.sink_name
                        ));
                        self.refresh();
                    }
                }
            }
            Panel::Sinks => {
                if let Some(sink) = self.selected_sink().cloned() {
                    if sink.is_virtual {
                        if let Err(e) = self.audio.remove_virtual_sink(&sink) {
                            self.status_msg = Some(format!("Error: {}", e));
                        } else {
                            self.status_msg =
                                Some(format!("Removed virtual sink '{}'", sink.name));
                            self.refresh();
                        }
                    } else {
                        self.status_msg = Some("Can only delete virtual sinks".to_string());
                    }
                }
            }
            _ => {
                self.status_msg = Some("Select a loopback or virtual sink to delete".to_string());
            }
        }
    }

    pub fn begin_create_virtual_source(&mut self) {
        if let Some(sink) = self.selected_sink().cloned() {
            let default_name = format!("{}-input", sink.name.replace('.', "-"));
            self.dialog = Dialog::CreateVirtualSource {
                sink,
                name: default_name,
            };
        } else {
            self.status_msg = Some("No sink selected".to_string());
        }
    }

    pub fn confirm_create_virtual_source(&mut self) {
        if let Dialog::CreateVirtualSource { ref sink, ref name } = self.dialog.clone() {
            let name = name.trim().to_string();
            if name.is_empty() {
                return;
            }
            let monitor = format!("{}.monitor", sink.name);
            match self.audio.create_virtual_source(&name, &monitor) {
                Ok(_) => {
                    self.status_msg = Some(format!(
                        "Created virtual input '{}' — select it in your recording app",
                        name
                    ));
                    self.dialog = Dialog::None;
                    self.refresh();
                }
                Err(e) => {
                    self.dialog = Dialog::Error(format!("{}", e));
                }
            }
        }
    }

    pub fn begin_listen_to_sink(&mut self) {
        if let Some(sink) = self.selected_sink().cloned() {
            self.dialog = Dialog::ListenToSink { sink };
            // Re-focus Sinks so the user picks the *output* sink
            self.focused = Panel::Sinks;
        } else {
            self.status_msg = Some("No sink selected".to_string());
        }
    }

    pub fn confirm_listen_to_sink(&mut self) {
        if let Dialog::ListenToSink { ref sink } = self.dialog.clone() {
            if let Some(output) = self.selected_sink().cloned() {
                // Construct a synthetic Source representing the monitor of the chosen sink.
                let monitor = Source {
                    name: format!("{}.monitor", sink.name),
                    description: format!("Monitor of {}", sink.description),
                    is_monitor: true,
                };
                match self.audio.create_loopback(&monitor, &output) {
                    Ok(lb) => {
                        self.status_msg = Some(format!(
                            "Listening: {} → {}",
                            sink.description, output.description
                        ));
                        self.dialog = Dialog::None;
                        self.focused = Panel::Loopbacks;
                        self.refresh();
                        let _ = lb;
                    }
                    Err(e) => {
                        self.dialog = Dialog::Error(format!("{}", e));
                    }
                }
            }
        }
    }

    pub fn begin_move_sink_input(&mut self) {
        if let Some(input) = self.selected_sink_input().cloned() {
            self.dialog = Dialog::MoveSinkInput { input };
            self.focused = Panel::Sinks;
        } else {
            self.status_msg = Some("No application selected".to_string());
        }
    }

    pub fn confirm_move_sink_input(&mut self) {
        if let Dialog::MoveSinkInput { ref input } = self.dialog.clone() {
            if let Some(sink) = self.selected_sink().cloned() {
                match self.audio.move_sink_input(input, &sink) {
                    Ok(()) => {
                        self.status_msg = Some(format!(
                            "Moved '{}' → {}",
                            input.app_name, sink.description
                        ));
                        self.dialog = Dialog::None;
                        self.focused = Panel::Applications;
                        self.refresh();
                    }
                    Err(e) => {
                        self.dialog = Dialog::Error(format!("{}", e));
                    }
                }
            }
        }
    }

    pub fn confirm_new_loopback(&mut self) {
        match self.dialog.clone() {
            Dialog::NewLoopbackPickSource => {
                if let Some(src) = self.selected_source().cloned() {
                    self.dialog = Dialog::NewLoopbackPickSink { source: src };
                }
            }
            Dialog::NewLoopbackPickSink { source } => {
                if let Some(sink) = self.selected_sink().cloned() {
                    match self.audio.create_loopback(&source, &sink) {
                        Ok(lb) => {
                            self.status_msg = Some(format!(
                                "Created loopback {} → {}",
                                lb.source_name, lb.sink_name
                            ));
                            self.dialog = Dialog::None;
                            self.refresh();
                        }
                        Err(e) => {
                            self.dialog = Dialog::Error(format!("{}", e));
                        }
                    }
                }
            }
            _ => {}
        }
    }

    pub fn confirm_new_virtual_sink(&mut self) {
        if let Dialog::NewVirtualSink { ref input } = self.dialog.clone() {
            let name = input.trim().to_string();
            if name.is_empty() {
                return;
            }
            match self.audio.create_virtual_sink(&name) {
                Ok(_) => {
                    self.status_msg = Some(format!("Created virtual sink '{}'", name));
                    self.dialog = Dialog::None;
                    self.refresh();
                }
                Err(e) => {
                    self.dialog = Dialog::Error(format!("{}", e));
                }
            }
        }
    }

    pub fn load_preset(&mut self, idx: usize) -> Result<()> {
        let preset = self.config.presets.get(idx).cloned();
        let Some(preset) = preset else {
            return Ok(());
        };
        for name in &preset.virtual_sinks {
            self.audio.create_virtual_sink(name)?;
        }
        self.refresh();
        for lb in &preset.loopbacks {
            let src = self
                .sources
                .iter()
                .find(|s| s.name == lb.source)
                .cloned();
            let snk = self.sinks.iter().find(|s| s.name == lb.sink).cloned();
            if let (Some(src), Some(snk)) = (src, snk) {
                self.audio.create_loopback(&src, &snk)?;
            }
        }
        self.refresh();
        self.status_msg = Some(format!("Loaded preset '{}'", preset.name));
        Ok(())
    }

    pub fn save_current_as_preset(&mut self, name: &str) {
        let name = name.trim().to_string();
        if name.is_empty() {
            return;
        }
        let preset = Preset {
            name: name.clone(),
            virtual_sinks: self
                .sinks
                .iter()
                .filter(|s| s.is_virtual)
                .map(|s| s.name.clone())
                .collect(),
            loopbacks: self
                .loopbacks
                .iter()
                .map(|lb| LoopbackEntry {
                    source: lb.source_name.clone(),
                    sink: lb.sink_name.clone(),
                })
                .collect(),
        };
        self.config.add_or_replace_preset(preset);
        if let Err(e) = self.config.save() {
            self.status_msg = Some(format!("Failed to save preset: {}", e));
        } else {
            self.status_msg = Some(format!("Saved preset '{}'", name));
        }
    }
}
