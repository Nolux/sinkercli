use std::cell::RefCell;
use std::process::Command;
use std::rc::Rc;

use anyhow::{anyhow, Result};
use libpulse_binding::callbacks::ListResult;
use libpulse_binding::context::{Context, FlagSet as ContextFlagSet, State};
use libpulse_binding::mainloop::standard::{IterateResult, Mainloop};

use super::{AudioBackend, Loopback, Sink, SinkInput, Source};

pub struct PulseBackend;

impl PulseBackend {
    pub fn new() -> Self {
        PulseBackend
    }
}

// Open a fresh mainloop+context pair and wait for Ready state.
fn connect() -> Result<(Mainloop, Context)> {
    let mut mainloop =
        Mainloop::new().ok_or_else(|| anyhow!("Failed to create PulseAudio mainloop"))?;
    let mut context = Context::new(&mainloop, "sinkercli")
        .ok_or_else(|| anyhow!("Failed to create PulseAudio context"))?;
    context
        .connect(None, ContextFlagSet::NOFLAGS, None)
        .map_err(|e| anyhow!("Failed to connect to PulseAudio: {:?}", e))?;

    loop {
        match mainloop.iterate(false) {
            IterateResult::Quit(_) | IterateResult::Err(_) => {
                return Err(anyhow!("Mainloop error during connect"));
            }
            IterateResult::Success(_) => {}
        }
        match context.get_state() {
            State::Ready => break,
            State::Failed | State::Terminated => {
                return Err(anyhow!("PulseAudio context failed to connect"));
            }
            _ => {}
        }
    }
    Ok((mainloop, context))
}

// Run the mainloop until the shared `done` flag is set to true.
fn run_until(mainloop: &mut Mainloop, done: &Rc<RefCell<bool>>) -> Result<()> {
    loop {
        match mainloop.iterate(false) {
            IterateResult::Quit(_) | IterateResult::Err(_) => {
                return Err(anyhow!("PulseAudio mainloop error"));
            }
            IterateResult::Success(_) => {}
        }
        if *done.borrow() {
            return Ok(());
        }
    }
}

impl AudioBackend for PulseBackend {
    fn sources(&self) -> Result<Vec<Source>> {
        let (mut mainloop, context) = connect()?;

        let done: Rc<RefCell<bool>> = Rc::new(RefCell::new(false));
        let data: Rc<RefCell<Vec<Source>>> = Rc::new(RefCell::new(Vec::new()));
        let done2 = done.clone();
        let data2 = data.clone();

        let _op = context.introspect().get_source_info_list(move |lr| match lr {
            ListResult::Item(info) => {
                let name = info.name.as_deref().unwrap_or("").to_string();
                let raw_desc = info.description.as_deref().unwrap_or(&name).to_string();
                // Label monitors clearly; keep them so users can route to recording software.
                let (description, is_monitor) = if name.ends_with(".monitor") {
                    (format!("[mon] {}", raw_desc), true)
                } else {
                    (raw_desc, false)
                };
                data2.borrow_mut().push(Source { name, description, is_monitor });
            }
            ListResult::End | ListResult::Error => *done2.borrow_mut() = true,
        });

        run_until(&mut mainloop, &done)?;
        Ok(data.borrow().clone())
    }

    fn sinks(&self) -> Result<Vec<Sink>> {
        let (mut mainloop, context) = connect()?;

        // Collect null-sink module IDs first.
        let done: Rc<RefCell<bool>> = Rc::new(RefCell::new(false));
        let null_mods: Rc<RefCell<Vec<u32>>> = Rc::new(RefCell::new(Vec::new()));
        let done2 = done.clone();
        let null2 = null_mods.clone();

        let _op = context.introspect().get_module_info_list(move |lr| match lr {
            ListResult::Item(info) => {
                if info.name.as_deref() == Some("module-null-sink") {
                    null2.borrow_mut().push(info.index);
                }
            }
            ListResult::End | ListResult::Error => *done2.borrow_mut() = true,
        });
        run_until(&mut mainloop, &done)?;
        drop(_op);

        let null_module_ids = null_mods.borrow().clone();

        // Now list sinks.
        let done: Rc<RefCell<bool>> = Rc::new(RefCell::new(false));
        let data: Rc<RefCell<Vec<Sink>>> = Rc::new(RefCell::new(Vec::new()));
        let done2 = done.clone();
        let data2 = data.clone();

        let _op = context.introspect().get_sink_info_list(move |lr| match lr {
            ListResult::Item(info) => {
                let name = info.name.as_deref().unwrap_or("").to_string();
                let description = info
                    .description
                    .as_deref()
                    .unwrap_or(&name)
                    .to_string();
                let is_virtual = info
                    .owner_module
                    .map(|mid| null_module_ids.contains(&mid))
                    .unwrap_or(false);
                data2.borrow_mut().push(Sink { name, description, is_virtual });
            }
            ListResult::End | ListResult::Error => *done2.borrow_mut() = true,
        });
        run_until(&mut mainloop, &done)?;
        Ok(data.borrow().clone())
    }

    fn loopbacks(&self) -> Result<Vec<Loopback>> {
        let (mut mainloop, context) = connect()?;

        let done: Rc<RefCell<bool>> = Rc::new(RefCell::new(false));
        let data: Rc<RefCell<Vec<Loopback>>> = Rc::new(RefCell::new(Vec::new()));
        let done2 = done.clone();
        let data2 = data.clone();

        let _op = context.introspect().get_module_info_list(move |lr| match lr {
            ListResult::Item(info) => {
                if info.name.as_deref() == Some("module-loopback") {
                    let args = info.argument.as_deref().unwrap_or("").to_string();
                    data2.borrow_mut().push(Loopback {
                        module_id: info.index,
                        source_name: extract_arg(&args, "source").unwrap_or_default(),
                        sink_name: extract_arg(&args, "sink").unwrap_or_default(),
                    });
                }
            }
            ListResult::End | ListResult::Error => *done2.borrow_mut() = true,
        });

        run_until(&mut mainloop, &done)?;
        Ok(data.borrow().clone())
    }

    fn create_loopback(&self, source: &Source, sink: &Sink) -> Result<Loopback> {
        let out = Command::new("pactl")
            .args([
                "load-module",
                "module-loopback",
                &format!("source={}", source.name),
                &format!("sink={}", sink.name),
                "latency_msec=50",
            ])
            .output()?;

        if !out.status.success() {
            return Err(anyhow!(
                "pactl load-module module-loopback failed: {}",
                String::from_utf8_lossy(&out.stderr)
            ));
        }

        let module_id: u32 = String::from_utf8_lossy(&out.stdout)
            .trim()
            .parse()
            .map_err(|_| anyhow!("Could not parse module ID from pactl output"))?;

        Ok(Loopback {
            module_id,
            source_name: source.name.clone(),
            sink_name: sink.name.clone(),
        })
    }

    fn create_virtual_sink(&self, name: &str) -> Result<Sink> {
        let out = Command::new("pactl")
            .args([
                "load-module",
                "module-null-sink",
                &format!("sink_name={}", name),
                &format!("sink_properties=device.description={}", name),
            ])
            .output()?;

        if !out.status.success() {
            return Err(anyhow!(
                "pactl load-module module-null-sink failed: {}",
                String::from_utf8_lossy(&out.stderr)
            ));
        }

        Ok(Sink { name: name.to_string(), description: name.to_string(), is_virtual: true })
    }

    fn remove_loopback(&self, lb: &Loopback) -> Result<()> {
        pactl_unload(lb.module_id)
    }

    fn remove_virtual_sink(&self, sink: &Sink) -> Result<()> {
        // Look up the module ID that owns this sink name via pactl.
        let out = Command::new("pactl").args(["list", "sinks"]).output()?;
        let text = String::from_utf8_lossy(&out.stdout);
        let mid = find_sink_module_id(&text, &sink.name)
            .ok_or_else(|| anyhow!("Could not find module for sink '{}'", sink.name))?;
        pactl_unload(mid)
    }

    fn sink_inputs(&self) -> Result<Vec<SinkInput>> {
        let (mut mainloop, context) = connect()?;

        // Build a map of sink index → sink name so we can label the current sink.
        let done: Rc<RefCell<bool>> = Rc::new(RefCell::new(false));
        let sink_names: Rc<RefCell<std::collections::HashMap<u32, String>>> =
            Rc::new(RefCell::new(std::collections::HashMap::new()));
        let done2 = done.clone();
        let sn2 = sink_names.clone();

        let _op = context.introspect().get_sink_info_list(move |lr| match lr {
            ListResult::Item(info) => {
                let name = info.name.as_deref().unwrap_or("").to_string();
                sn2.borrow_mut().insert(info.index, name);
            }
            ListResult::End | ListResult::Error => *done2.borrow_mut() = true,
        });
        run_until(&mut mainloop, &done)?;
        drop(_op);

        let sink_map = sink_names.borrow().clone();

        let done: Rc<RefCell<bool>> = Rc::new(RefCell::new(false));
        let data: Rc<RefCell<Vec<SinkInput>>> = Rc::new(RefCell::new(Vec::new()));
        let done2 = done.clone();
        let data2 = data.clone();

        let _op = context
            .introspect()
            .get_sink_input_info_list(move |lr| match lr {
                ListResult::Item(info) => {
                    let app_name = info
                        .proplist
                        .get_str("application.name")
                        .or_else(|| info.name.as_deref().map(str::to_string))
                        .unwrap_or_else(|| format!("stream {}", info.index));
                    let current_sink_name = sink_map
                        .get(&info.sink)
                        .cloned()
                        .unwrap_or_default();
                    data2.borrow_mut().push(SinkInput { id: info.index, app_name, current_sink_name });
                }
                ListResult::End | ListResult::Error => *done2.borrow_mut() = true,
            });

        run_until(&mut mainloop, &done)?;
        Ok(data.borrow().clone())
    }

    fn create_virtual_source(&self, name: &str, monitor_source: &str) -> Result<Source> {
        let out = Command::new("pactl")
            .args([
                "load-module",
                "module-virtual-source",
                &format!("source_name={}", name),
                &format!("master={}", monitor_source),
                &format!("source_properties=device.description={}", name),
            ])
            .output()?;

        if !out.status.success() {
            return Err(anyhow!(
                "pactl load-module module-virtual-source failed: {}",
                String::from_utf8_lossy(&out.stderr)
            ));
        }

        Ok(Source { name: name.to_string(), description: name.to_string(), is_monitor: false })
    }

    fn move_sink_input(&self, input: &SinkInput, sink: &Sink) -> Result<()> {
        let out = Command::new("pactl")
            .args([
                "move-sink-input",
                &input.id.to_string(),
                &sink.name,
            ])
            .output()?;
        if !out.status.success() {
            return Err(anyhow!(
                "pactl move-sink-input failed: {}",
                String::from_utf8_lossy(&out.stderr)
            ));
        }
        Ok(())
    }
}

fn pactl_unload(module_id: u32) -> Result<()> {
    let out = Command::new("pactl")
        .args(["unload-module", &module_id.to_string()])
        .output()?;
    if !out.status.success() {
        return Err(anyhow!(
            "pactl unload-module {} failed: {}",
            module_id,
            String::from_utf8_lossy(&out.stderr)
        ));
    }
    Ok(())
}

fn extract_arg(args: &str, key: &str) -> Option<String> {
    for part in args.split_whitespace() {
        if let Some(val) = part.strip_prefix(&format!("{}=", key)) {
            return Some(val.trim_matches('"').to_string());
        }
    }
    None
}

fn find_sink_module_id(text: &str, sink_name: &str) -> Option<u32> {
    let mut in_target = false;
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.contains(&format!("Name: {}", sink_name)) {
            in_target = true;
        }
        if in_target {
            if let Some(rest) = trimmed.strip_prefix("Owner Module:") {
                let id_str = rest.trim();
                if id_str != "n/a" {
                    return id_str.parse().ok();
                }
            }
            if trimmed.starts_with("Sink #") {
                in_target = false;
            }
        }
    }
    None
}
