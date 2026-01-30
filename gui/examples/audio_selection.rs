use anyhow::{Context, Result};
use clap::Parser;
use gui::widgets::{Dropdown, Label, Widget};
use pal::{
    audio::{AudioBackend, AudioStream, DeviceInfo, StreamConfig},
    App, NativeWindow, UIEvent,
};
use standalone::audio;
// unused imports removed
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

// Helper for window handle
struct WindowHandleWrapper<'a>(&'a dyn pal::NativeWindow);
impl<'a> raw_window_handle::HasWindowHandle for WindowHandleWrapper<'a> {
    fn window_handle(
        &self,
    ) -> Result<raw_window_handle::WindowHandle<'_>, raw_window_handle::HandleError> {
        Ok(unsafe { raw_window_handle::WindowHandle::borrow_raw(self.0.get_raw_handle()) })
    }
}
impl<'a> raw_window_handle::HasDisplayHandle for WindowHandleWrapper<'a> {
    fn display_handle(
        &self,
    ) -> Result<raw_window_handle::DisplayHandle<'_>, raw_window_handle::HandleError> {
        Ok(
            unsafe {
                raw_window_handle::DisplayHandle::borrow_raw(self.0.get_raw_display_handle())
            },
        )
    }
}

struct MockAudioStream;
impl AudioStream for MockAudioStream {
    fn play(&self) -> Result<()> {
        Ok(())
    }
    fn pause(&self) -> Result<()> {
        Ok(())
    }
}

struct MockAudioBackend;
impl AudioBackend for MockAudioBackend {
    fn enumerate_input_devices(&self) -> Vec<DeviceInfo> {
        vec![]
    }
    fn enumerate_output_devices(&self) -> Vec<DeviceInfo> {
        vec![
            DeviceInfo {
                name: "Speakers".to_string(),
                id: "mock-speakers".to_string(),
            },
            DeviceInfo {
                name: "Headphones 1".to_string(),
                id: "mock-hp-1".to_string(),
            },
            DeviceInfo {
                name: "Headphones 2".to_string(),
                id: "mock-hp-2".to_string(),
            },
        ]
    }
    fn create_input_stream(
        &self,
        _device_id: &str,
        _config: &StreamConfig,
        _callback: Box<dyn FnMut(&[f32]) + Send>,
    ) -> Result<Box<dyn AudioStream>> {
        Ok(Box::new(MockAudioStream))
    }
    fn create_output_stream(
        &self,
        _device_id: &str,
        _config: &StreamConfig,
        _callback: Box<dyn FnMut(&mut [f32]) + Send>,
    ) -> Result<Box<dyn AudioStream>> {
        Ok(Box::new(MockAudioStream))
    }
}

#[derive(Parser, Debug)]
struct Args {
    #[arg(long)]
    debug_server: bool,
}

fn main() -> Result<()> {
    // Init Logging
    let stdout_layer = tracing_subscriber::fmt::layer()
        .with_writer(std::io::stdout)
        .with_target(false)
        .with_level(true);
    tracing_subscriber::registry().with(stdout_layer).init();

    let args = Args::parse();

    // Init App & Window
    let app: Box<dyn App> = Box::new(pal::AppImpl::init()?);
    let mut window: Box<dyn NativeWindow> =
        Box::new(unsafe { pal::Window::attach(std::ptr::null_mut())? });
    window.set_size(600, 400)?;

    let window_handle = WindowHandleWrapper(&*window);
    let mut renderer =
        gui::Renderer::new(&window_handle, 600, 400).context("Failed to init renderer")?;

    // Audio Host with Mock Backend
    let mut audio_host = audio::StandaloneAudioHost::new_with_backend(Box::new(MockAudioBackend))
        .context("Failed to init audio")?;

    // UI Setup
    let mut widgets = gui::widgets::container::WidgetContainer::new();
    let layout = gui::widgets::layout::FlexLayout::new(gui::widgets::layout::FlexDirection::Column)
        .with_spacing(20.0)
        .with_padding(40.0)
        .with_align(gui::widgets::layout::AlignItems::Center);
    widgets.set_layout(Box::new(layout));

    // 1. Label
    let label = Label::new("Select Audio Output Device", 0.0, 0.0, 300.0, 30.0).centered(true);
    widgets.add_widget(Box::new(label));

    // 2. Dropdown
    let output_devices = audio_host.get_backend().enumerate_output_devices();
    let device_names: Vec<String> = output_devices.iter().map(|d| d.name.clone()).collect();

    let mut dropdown = Dropdown::new(0.0, 0.0, 300.0, 40.0);
    dropdown.set_items(device_names.clone());

    // Select default
    let initial_idx = if let Some(first) = output_devices.first() {
        device_names
            .iter()
            .position(|n| n == &first.name)
            .unwrap_or(0)
    } else {
        0
    };
    dropdown.set_selected_index(initial_idx);

    let dropdown_id = dropdown.id();
    widgets.add_widget(Box::new(dropdown));

    // 3. Status Label
    let status_label = Label::new("Status: Ready", 0.0, 0.0, 300.0, 30.0).centered(true);
    let status_id = status_label.id();
    widgets.add_widget(Box::new(status_label));

    widgets.apply_layout(600.0, 400.0);

    // Initial Audio Start
    if let Some(device) = output_devices.first() {
        tracing::info!("Starting audio on: {}", device.name);
        audio_host.start_audio(
            &device.name,
            44100,
            512,
            audio_core::processor::NoOpProcessor,
        )?;
        if let Some(w) = widgets.get_widget_mut(status_id) {
            if let Some(lbl) = w.as_any_mut().downcast_mut::<Label>() {
                lbl.set_text(format!("Playing on: {}", device.name));
            }
        }
    }

    // Event Loop
    let (app_tx, app_rx) = std::sync::mpsc::channel();
    let app_tx_cb = app_tx.clone();
    window.event_router().set_callback(move |event| {
        tracing::info!("App Callback received event: {:?}", event);
        let _ = app_tx_cb.send(event);
    });

    // RPC Server
    let (_lockfile_guard, ready_notify, debug_rx) = if args.debug_server {
        use crossbeam_channel;
        let (tx, rx) = crossbeam_channel::unbounded();
        let debug_rx = rx.clone();
        window.event_router().set_event_receiver(rx);
        let pid = std::process::id();
        let temp_dir = std::env::temp_dir();
        let path = temp_dir.join(format!("splug_pid_{}.json", pid));
        match debug_server::RpcServer::start(0, tx) {
            Ok((port, notify)) => {
                let json = format!("{{ \"port\": {}, \"pid\": {} }}", port, pid);
                std::fs::write(&path, json)?;
                (Some(path), Some(notify), Some(debug_rx))
            }
            Err(_) => (None, None, None),
        }
    } else {
        (None, None, None)
    };

    if let Some(notify) = ready_notify {
        notify.notify_one();
    }

    tracing::info!("Starting loop...");
    loop {
        if let Some(rx) = &debug_rx {
            if !rx.is_empty() {
                tracing::info!("DEBUG RX has {} events!", rx.len());
            }
        }
        window.event_router().poll_events();
        app.poll_events();

        while let Ok(event) = app_rx.try_recv() {
            match event {
                UIEvent::Quit => return Ok(()),
                UIEvent::Resize(w, h) => {
                    let _ = window.set_size(w, h);
                    let _ = renderer.resize(w, h);
                    widgets.apply_layout(w as f32, h as f32);
                }
                UIEvent::CaptureScreen(reply_tx) => {
                    tracing::info!("Capturing screen...");
                    if let Ok(image) = renderer.capture_frame() {
                        let width = image.width();
                        let height = image.height();
                        let data = image.into_raw();
                        let _ = reply_tx.send((data, width, height));
                    } else {
                        tracing::error!("Failed to capture frame");
                    }
                }
                evt => {
                    let results = widgets.handle_ui_event(evt);
                    for (id, res) in results {
                        if id == dropdown_id {
                            if let gui::widgets::EventResult::ValueChanged(_) = res {
                                // Handle change
                                // Get selected item name first (immutable borrow)
                                let selected_name = if let Some(w) = widgets.get_widget(dropdown_id)
                                {
                                    if let Some(dd) = w.as_any().downcast_ref::<Dropdown>() {
                                        dd.selected_item().map(|s| s.to_string())
                                    } else {
                                        None
                                    }
                                } else {
                                    None
                                };

                                // Use it (mutable borrow later)
                                if let Some(name) = selected_name {
                                    tracing::info!("Switching to device: {}", name);
                                    let _ = audio_host.start_audio(
                                        &name,
                                        44100,
                                        512,
                                        audio_core::processor::NoOpProcessor,
                                    );

                                    if let Some(w) = widgets.get_widget_mut(status_id) {
                                        if let Some(lbl) = w.as_any_mut().downcast_mut::<Label>() {
                                            lbl.set_text(format!("Playing on: {}", name));
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        let _ = renderer.draw_frame(Some(&widgets));
        std::thread::sleep(std::time::Duration::from_millis(16));
    }
}
