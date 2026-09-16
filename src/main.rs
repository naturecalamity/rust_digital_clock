use chrono::{Local, NaiveTime, Timelike, Utc};
use chrono_tz::TZ_VARIANTS;
use chrono_tz::Tz;
use eframe::egui;
use rodio::{OutputStream, Sink, Source};
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::BufReader;
use std::time::Duration;

#[derive(Serialize, Deserialize, Clone)]
struct Alarm {
    id: usize,
    time: NaiveTime,
    label: String,
    enabled: bool,
}

#[derive(Serialize, Deserialize, Clone)]
struct ScheduledTask {
    id: usize,
    time: NaiveTime,
    title: String,
    completed: bool,
}

#[derive(Serialize, Deserialize, Default)]
struct SavedData {
    alarms: Vec<Alarm>,
    schedules: Vec<ScheduledTask>,
}

struct ClockApp {
    selected_tz: Tz,
    alarms: Vec<Alarm>,
    new_alarm_time: String,
    new_alarm_label: String,
    schedules: Vec<ScheduledTask>,
    new_task_time: String,
    new_task_title: String,
    next_id: usize,
    active_alarm_msg: Option<String>,
}

impl Default for ClockApp {
    fn default() -> Self {
        let mut app = Self {
            selected_tz: chrono_tz::UTC,
            alarms: Vec::new(),
            new_alarm_time: "07:00".to_string(),
            new_alarm_label: "Wake Up".to_string(),
            schedules: Vec::new(),
            new_task_time: "09:00".to_string(),
            new_task_title: "Team Sync".to_string(),
            next_id: 1,
            active_alarm_msg: None,
        };
        app.load_data();
        app
    }
}

impl ClockApp {
    fn save_data(&self) {
        let data = SavedData {
            alarms: self.alarms.clone(),
            schedules: self.schedules.clone(),
        };
        if let Ok(file) = File::create("clock_data.json") {
            let _ = serde_json::to_writer_pretty(file, &data);
        }
    }

    fn load_data(&mut self) {
        if let Ok(file) = File::open("clock_data.json") {
            let reader = BufReader::new(file);
            if let Ok(data) = serde_json::from_reader::<_, SavedData>(reader) {
                self.alarms = data.alarms;
                self.schedules = data.schedules;
                self.next_id = self
                    .alarms
                    .iter()
                    .map(|a| a.id)
                    .chain(self.schedules.iter().map(|t| t.id))
                    .max()
                    .unwrap_or(0)
                    + 1;
            }
        }
    }

    fn check_triggers(&mut self) {
        let current_time = Local::now().time();

        if current_time.second() == 0 {
            for alarm in &self.alarms {
                if alarm.enabled
                    && alarm.time.hour() == current_time.hour()
                    && alarm.time.minute() == current_time.minute()
                {
                    self.active_alarm_msg = Some(format!("⏰ ALARM: {}", alarm.label));
                    play_beep_sound();
                }
            }
        }
    }
}

impl eframe::App for ClockApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        ctx.request_repaint_after(Duration::from_millis(500));
        self.check_triggers();

        let local_now = Local::now();

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("🕒 Rust Digital Clock & Planner");
            ui.separator();

            // --- CLOCK & WORLD TIME ---
            ui.horizontal(|ui| {
                ui.group(|ui| {
                    ui.vertical(|ui| {
                        ui.label(egui::RichText::new("Local Time").strong());
                        ui.label(
                            egui::RichText::new(local_now.format("%H:%M:%S").to_string())
                                .size(32.0)
                                .monospace(),
                        );
                        ui.label(local_now.format("%A, %B %d, %Y").to_string());
                    });
                });

                let tz_now = Utc::now().with_timezone(&self.selected_tz);
                ui.group(|ui| {
                    ui.vertical(|ui| {
                        ui.label(
                            egui::RichText::new(format!(
                                "World Clock ({})",
                                self.selected_tz.name()
                            ))
                            .strong(),
                        );
                        ui.label(
                            egui::RichText::new(tz_now.format("%H:%M:%S").to_string())
                                .size(32.0)
                                .monospace(),
                        );
                        ui.label(tz_now.format("%A, %B %d, %Y").to_string());
                    });
                });
            });

            // Timezone Picker
            ui.add_space(5.0);
            ui.horizontal(|ui| {
                ui.label("Select Timezone:");
                egui::ComboBox::from_id_source("tz_picker")
                    .selected_text(self.selected_tz.name())
                    .show_ui(ui, |ui| {
                        for tz in TZ_VARIANTS.iter() {
                            ui.selectable_value(&mut self.selected_tz, *tz, tz.name());
                        }
                    });
            });

            ui.separator();

            // --- ACTIVE ALARM BANNER ---
            if let Some(msg) = &self.active_alarm_msg.clone() {
                ui.colored_label(
                    egui::Color32::RED,
                    egui::RichText::new(msg).size(18.0).strong(),
                );
                if ui.button("Dismiss").clicked() {
                    self.active_alarm_msg = None;
                }
                ui.separator();
            }

            let mut state_changed = false;

            // --- ALARMS ---
            ui.collapsing("⏰ Alarms", |ui| {
                ui.horizontal(|ui| {
                    ui.label("Time (HH:MM):");
                    ui.text_edit_singleline(&mut self.new_alarm_time);
                    ui.label("Label:");
                    ui.text_edit_singleline(&mut self.new_alarm_label);
                    if ui.button("Add").clicked() {
                        if let Ok(time) = NaiveTime::parse_from_str(&self.new_alarm_time, "%H:%M") {
                            self.alarms.push(Alarm {
                                id: self.next_id,
                                time,
                                label: self.new_alarm_label.clone(),
                                enabled: true,
                            });
                            self.next_id += 1;
                            state_changed = true;
                        }
                    }
                });

                let mut to_remove = None;
                for (idx, alarm) in self.alarms.iter_mut().enumerate() {
                    ui.horizontal(|ui| {
                        if ui.checkbox(&mut alarm.enabled, "").changed() {
                            state_changed = true;
                        }
                        ui.label(format!("{} - {}", alarm.time.format("%H:%M"), alarm.label));
                        if ui.button("🗑").clicked() {
                            to_remove = Some(idx);
                        }
                    });
                }
                if let Some(idx) = to_remove {
                    self.alarms.remove(idx);
                    state_changed = true;
                }
            });

            // --- SCHEDULES ---
            ui.collapsing("📅 Daily Schedule", |ui| {
                ui.horizontal(|ui| {
                    ui.label("Time (HH:MM):");
                    ui.text_edit_singleline(&mut self.new_task_time);
                    ui.label("Task:");
                    ui.text_edit_singleline(&mut self.new_task_title);
                    if ui.button("Add").clicked() {
                        if let Ok(time) = NaiveTime::parse_from_str(&self.new_task_time, "%H:%M") {
                            self.schedules.push(ScheduledTask {
                                id: self.next_id,
                                time,
                                title: self.new_task_title.clone(),
                                completed: false,
                            });
                            self.next_id += 1;
                            state_changed = true;
                        }
                    }
                });

                let mut to_remove = None;
                for (idx, task) in self.schedules.iter_mut().enumerate() {
                    ui.horizontal(|ui| {
                        if ui.checkbox(&mut task.completed, "").changed() {
                            state_changed = true;
                        }
                        let text = format!("[{}] {}", task.time.format("%H:%M"), task.title);
                        if task.completed {
                            ui.label(egui::RichText::new(text).strikethrough());
                        } else {
                            ui.label(text);
                        }
                        if ui.button("🗑").clicked() {
                            to_remove = Some(idx);
                        }
                    });
                }
                if let Some(idx) = to_remove {
                    self.schedules.remove(idx);
                    state_changed = true;
                }
            });

            if state_changed {
                self.save_data();
            }
        });
    }
}

fn play_beep_sound() {
    std::thread::spawn(|| {
        if let Ok((_stream, stream_handle)) = OutputStream::try_default() {
            if let Ok(sink) = Sink::try_new(&stream_handle) {
                let source =
                    rodio::source::SineWave::new(440.0).take_duration(Duration::from_secs(2));
                sink.append(source);
                sink.sleep_until_end();
            }
        }
    });
}

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([500.0, 480.0]),
        ..Default::default()
    };
    eframe::run_native(
        "Rust Digital Clock & Scheduler",
        options,
        Box::new(|_cc| Box::new(ClockApp::default())),
    )
}
