//! egui controls that write DSP parameters into a shared engine.

use eframe::egui;
use minimix_config::{default_config_path, Preset, Settings};
use minimix_engine::Engine;
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};

/// Desktop parameter surface.
pub struct MinimixApp {
    settings: Settings,
    engine: Arc<Mutex<Engine>>,
    status: String,
}

impl MinimixApp {
    /// Build UI bound to a live `Engine`.
    #[must_use]
    pub fn new(engine: Arc<Mutex<Engine>>) -> Self {
        let path = default_config_path();
        let settings = Settings::load(&path).unwrap_or_default();
        let mut app = Self {
            settings,
            engine,
            status: format!("config={}", path.display()),
        };
        app.push_to_engine();
        app
    }

    fn push_to_engine(&mut self) {
        let Ok(mut eng) = self.engine.lock() else {
            self.status = "engine lock poisoned".into();
            return;
        };
        apply_preset(&mut eng, &self.settings.active);
    }

    fn save(&mut self) {
        let path = default_config_path();
        match self.settings.save(&path) {
            Ok(()) => self.status = format!("saved {}", path.display()),
            Err(err) => self.status = format!("save failed: {err}"),
        }
    }
}

impl eframe::App for MinimixApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Minimix");
            ui.label("Controls the realtime engine. This thread never touches PCM.");
            ui.separator();

            {
                let active = &mut self.settings.active;
                ui.checkbox(&mut active.bypass, "Bypass");
                ui.add(egui::Slider::new(&mut active.output_gain, 0.0..=2.0).text("Output gain"));
                ui.add(
                    egui::Slider::new(&mut active.highpass_hz, 20.0..=400.0).text("High-pass Hz"),
                );
                ui.add(
                    egui::Slider::new(&mut active.peak_gain_db[0], -12.0..=12.0)
                        .text("Low band dB"),
                );
                ui.add(
                    egui::Slider::new(&mut active.peak_gain_db[1], -12.0..=12.0)
                        .text("Presence dB"),
                );
                ui.add(egui::Slider::new(&mut active.peak_gain_db[2], -12.0..=12.0).text("Air dB"));
                ui.add(
                    egui::Slider::new(&mut active.comp_threshold_db, -40.0..=0.0)
                        .text("Comp threshold dB"),
                );
                ui.add(egui::Slider::new(&mut active.comp_ratio, 1.0..=20.0).text("Comp ratio"));
            }
            self.push_to_engine();

            ui.horizontal(|ui| {
                if ui.button("Save preset").clicked() {
                    let stored = self.settings.active.clone();
                    self.settings.presets.push(stored);
                    self.save();
                }
                if ui.button("Save settings").clicked() {
                    self.save();
                }
            });

            ui.separator();
            ui.label("Presets");
            let mut load_idx: Option<usize> = None;
            for (i, preset) in self.settings.presets.iter().enumerate() {
                if ui.selectable_label(false, &preset.name).clicked() {
                    load_idx = Some(i);
                }
            }
            if let Some(i) = load_idx {
                self.settings.active = self.settings.presets[i].clone();
                self.push_to_engine();
                self.status = format!("loaded {}", self.settings.active.name);
            }

            ui.separator();
            ui.weak(&self.status);
        });
    }
}

/// Apply a preset to the engine (atomics + filter coeffs).
pub fn apply_preset(engine: &mut Engine, preset: &Preset) {
    engine
        .params()
        .bypass
        .store(preset.bypass, Ordering::Relaxed);
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let q16 = (preset.output_gain.clamp(0.0, 4.0) * 65536.0) as u32;
    engine
        .params()
        .output_gain_q16
        .store(q16, Ordering::Relaxed);
    engine
        .processor_mut()
        .equalizer_mut()
        .set_highpass(preset.highpass_hz, 0.707);
    engine
        .processor_mut()
        .equalizer_mut()
        .set_peak(0, 250.0, 1.0, preset.peak_gain_db[0]);
    engine
        .processor_mut()
        .equalizer_mut()
        .set_peak(1, 1000.0, 1.0, preset.peak_gain_db[1]);
    engine
        .processor_mut()
        .equalizer_mut()
        .set_peak(2, 4000.0, 1.0, preset.peak_gain_db[2]);
    engine.processor_mut().compressor_mut().set_dynamics(
        preset.comp_threshold_db,
        preset.comp_ratio,
        1.0,
    );
}

/// Run the native window until closed.
///
/// # Errors
///
/// Returns when egui cannot start the window.
pub fn run(engine: Arc<Mutex<Engine>>) -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([420.0, 560.0])
            .with_title("Minimix"),
        ..Default::default()
    };
    eframe::run_native(
        "Minimix",
        options,
        Box::new(move |_cc| Ok(Box::new(MinimixApp::new(engine)))),
    )
}
