// This file defines the user interfaces for the application

use crate::config::Config;
use crate::utils::{format_duration, format_split_time, format_timer};

use glib::subclass::signal::SignalBuilder;
use std::sync::{Arc, RwLock};
use std::time::Duration;
use time::error::DifferentVariant;
use time::{Duration as TimeDuration, Time};

use adw::prelude::*;
use adw::{self, ApplicationWindow, Clamp, ToolbarView};
use glib::ControlFlow::Continue;
use gtk4::subclass::button;
use gtk4::Orientation::{Horizontal, Vertical};
use gtk4::{Align, Box as GtkBox, CenterBox, Label, ListBox};

use livesplit_core::{Run, Segment, TimeSpan, TimeStamp, Timer, TimerPhase};

use tracing::debug;

// Main screen for load / create splits
pub struct MainUI {}

// Timer layout for runs
pub struct TimerUI {
    timer: Arc<RwLock<Timer>>,
    config: Arc<RwLock<Config>>,
}

// Splits editor/Creator
pub struct EditorUI {}

pub struct SettingsUI {}

pub struct AboutUI {}

pub struct HelpUI {}

impl TimerUI {
    pub fn new(timer: Arc<RwLock<Timer>>, config: Arc<RwLock<Config>>) -> Self {
        Self { timer, config }
    }

    pub fn build_ui(&self) -> ToolbarView {
        // --- Root Clamp ---
        let clamp = Clamp::builder().maximum_size(300).build();

        // === Outer VBox ===
        let livesplit_gtk = GtkBox::builder()
            .orientation(Vertical)
            .valign(Align::Center)
            .halign(Align::Center)
            .width_request(300)
            .margin_top(24)
            .margin_bottom(24)
            .margin_start(24)
            .margin_end(24)
            .spacing(20)
            .build();

        // =====================
        // Run Info Section
        // =====================
        let run_info = TimerUI::build_run_info(&self.timer.read().unwrap());

        //
        // Splits List
        // =====================
        let splits = ListBox::new();
        splits.add_css_class("boxed-list");
        let splits_rows =
            TimerUI::build_splits_list(&self.timer.read().unwrap(), &self.config.read().unwrap());
        for row in splits_rows {
            splits.append(&row);
        }

        // =====================
        // Current Split + Timer
        // =====================
        let center_box = CenterBox::builder()
            .orientation(Horizontal)
            .margin_start(18)
            .margin_end(18)
            .build();
        center_box.set_start_widget(Some(&TimerUI::build_center_box_current_split_info(
            &self.timer.read().unwrap(),
            &self.config.read().unwrap(),
        )));
        center_box.set_end_widget(Some(&TimerUI::build_center_box_timer(
            &self.timer.read().unwrap(),
            &self.config.read().unwrap(),
        )));

        let splits_binding = splits.clone();
        let center_box_binding = center_box.clone();

        let timer_binding = self.timer.clone();
        let config_binding = self.config.clone();

        glib::timeout_add_local(Duration::from_millis(16), move || {
            let t = timer_binding.read().unwrap();
            let c = config_binding.read().unwrap();
            // =====================
            // Splits List
            // =====================
            // Remove all existing rows
            for (index, _) in t.run().segments().iter().enumerate() {
                if let Some(row) = splits_binding.row_at_index(0) {
                    splits_binding.remove(&row);
                }
            }
            // Now rebuild
            let splits_rows = TimerUI::build_splits_list(&t, &c);
            for row in splits_rows {
                splits_binding.append(&row);
            }

            // =====================
            // Current Split + Timer
            // =====================
            center_box_binding
                .set_start_widget(Some(&TimerUI::build_center_box_current_split_info(&t, &c)));
            center_box_binding.set_end_widget(Some(&TimerUI::build_center_box_timer(&t, &c)));

            Continue
        });

        // =====================
        // Assemble everything
        // =====================
        livesplit_gtk.append(&run_info);
        livesplit_gtk.append(&splits);
        livesplit_gtk.append(&center_box);

        clamp.set_child(Some(&livesplit_gtk));

        // Building the window
        let view = ToolbarView::new();
        let header = adw::HeaderBar::builder()
            .title_widget(&Label::new(Some("LiveSplit GTK")))
            .show_end_title_buttons(true)
            .build();
        view.add_top_bar(&header);
        view.set_content(Some(&clamp));

        view
    }
}

impl TimerUI {
    fn build_run_info(timer: &Timer) -> GtkBox {
        let run_info = GtkBox::builder()
            .orientation(Vertical)
            .halign(Align::Center)
            .build();

        let run_name = Label::builder().label(timer.run().game_name()).build();
        run_name.add_css_class("title-2");
        debug!("Run Name: {}", run_name.label());

        let category = Label::builder().label(timer.run().category_name()).build();
        category.add_css_class("heading");
        debug!("Category: {}", category.label());

        run_info.append(&run_name);
        run_info.append(&category);
        run_info
    }

    fn build_splits_list(timer: &Timer, config: &Config) -> Vec<adw::ActionRow> {
        let mut rows = Vec::new();

        let segments = timer.run().segments();
        let opt_current_segment_index = timer.current_split_index();

        for (index, segment) in segments.iter().enumerate() {
            let title = segment.name();

            let mut value;

            let segment_comparison = segment
                .comparison(timer.current_comparison())
                .real_time
                .unwrap_or_default()
                .to_duration();

            value = format_split_time(
                &segment.comparison(timer.current_comparison()),
                &timer,
                &config,
            );

            let mut segment_classes = Vec::new();
            let mut label_classes = Vec::new();

            if let Some(current_segment_index) = opt_current_segment_index {
                let goldsplit = segment.best_segment_time().real_time.unwrap_or_default();

                let previous_comparison_duration = if index > 0 {
                    segments
                        .get(index - 1)
                        .unwrap()
                        .comparison(timer.current_comparison())
                        .real_time
                        .unwrap_or_default()
                        .to_duration()
                } else {
                    TimeDuration::ZERO
                };

                let previous_comparison_time = if index > 0 {
                    segments
                        .get(index - 1)
                        .unwrap()
                        .split_time()
                        .real_time
                        .unwrap_or_default()
                        .to_duration()
                } else {
                    TimeDuration::ZERO
                };

                let segment_comparison_duration = segment_comparison
                    .checked_sub(previous_comparison_duration)
                    .unwrap_or_default()
                    .abs(); // Abs because later split might be shorter than previous

                if current_segment_index == index {
                    segment_classes.push("current-segment");

                    let current_dur = timer
                        .current_attempt_duration()
                        .to_duration()
                        .checked_add(timer.run().offset().to_duration())
                        .unwrap_or_default();
                    let paused_time = timer.get_pause_time().unwrap_or_default().to_duration();
                    let loading_times = if timer.current_timing_method()
                        == livesplit_core::TimingMethod::GameTime
                    {
                        timer.loading_times().to_duration()
                    } else {
                        TimeDuration::ZERO
                    };

                    let current_duration = current_dur
                        .checked_sub(paused_time)
                        .unwrap_or_default()
                        .checked_sub(loading_times)
                        .unwrap_or_default();

                    let diff = current_duration // Represents the time difference to comparison.
                        .checked_sub(segment_comparison)
                        .unwrap_or_default();

                    // We will calculate how long the split has been running to either show diff or comparison
                    let split_running_time = if index == 0 {
                        current_duration
                    } else {
                        assert!(current_duration > previous_comparison_time);
                        current_duration
                            .checked_sub(previous_comparison_time)
                            .unwrap_or_default()
                    };

                    if diff.is_positive()
                        || (goldsplit.to_duration() != TimeDuration::ZERO
                            && split_running_time >= goldsplit.to_duration())
                    {
                        let sign = if diff.is_positive() {
                            "+"
                        } else if diff.is_negative() {
                            "-"
                        } else {
                            "~"
                        };

                        let abs = diff.abs();
                        let formatted = format_duration(&abs);
                        value = format!("{}{}", sign, formatted);

                        label_classes = TimerUI::calculate_split_label_classes(
                            &timer,
                            segment_comparison_duration,
                            split_running_time,
                            diff,
                            goldsplit.to_duration(),
                            true,
                        );
                    }
                }
                if current_segment_index > index {
                    // If not current index or current index is not close to gold
                    let split_time = segment
                        .split_time()
                        .real_time
                        .unwrap_or_default()
                        .to_duration();

                    let diff = split_time
                        .checked_sub(segment_comparison)
                        .unwrap_or_default();

                    if config.general.split_format == Some(String::from("Time")) {
                        value = format_split_time(&segment.split_time(), &timer, &config);
                    } else {
                        // DIFF
                        let sign = if diff.is_positive() {
                            "+"
                        } else if diff.is_negative() {
                            "-"
                        } else {
                            "~"
                        };
                        let abs = diff.abs();
                        let formatted = format_duration(&abs);
                        value = format!("{}{}", sign, formatted);
                    }

                    label_classes = TimerUI::calculate_split_label_classes(
                        &timer,
                        segment_comparison_duration,
                        split_time
                            .checked_sub(previous_comparison_time)
                            .unwrap_or_default(),
                        diff,
                        goldsplit.to_duration(),
                        false,
                    );
                }
            }

            rows.push(Self::make_split_row(
                title,
                &value,
                &segment_classes,
                &label_classes,
            ));
        }

        rows
    }

    fn build_center_box_current_split_info(timer: &Timer, config: &Config) -> GtkBox {
        // Left side: current split info
        let current_split = GtkBox::builder().orientation(Vertical).build();

        let segments = timer.run().segments();
        let current_index = timer.current_split_index().unwrap_or(0);
        let current_segment = timer.current_split().unwrap_or(segments.get(0).unwrap());

        let previous_comparison_time = if current_index > 0 {
            segments
                .get(current_index - 1)
                .unwrap()
                .comparison(timer.current_comparison())
                .real_time
                .unwrap_or_default()
                .to_duration()
        } else {
            TimeDuration::ZERO
        };

        // Best
        let best_box = GtkBox::builder()
            .orientation(Horizontal)
            .margin_top(6)
            .spacing(2)
            .halign(Align::Start)
            .build();
        let best_label = Label::builder().label("Best:").build();
        best_label.add_css_class("caption-heading");

        let best_value = Label::builder()
            .label(format_split_time(
                &current_segment.best_segment_time(),
                &timer,
                &config,
            ))
            .build();
        best_value.add_css_class("caption");
        best_value.add_css_class("timer");
        best_box.append(&best_label);
        best_box.append(&best_value);

        // Comparison
        let comparison_box = GtkBox::builder()
            .orientation(Horizontal)
            .spacing(2)
            .halign(Align::Start)
            .build();
        let comparison_label = Label::builder() // TODO: Map comparisons to simpler string representations
            .label(format!(
                "{}:",
                config
                    .general
                    .comparison
                    .as_ref()
                    .unwrap_or(&String::from("PB"))
            ))
            .build();
        comparison_label.add_css_class("caption-heading");

        let comparison_value = Label::builder()
            .label(format_duration(
                &current_segment
                    .comparison(timer.current_comparison())
                    .real_time
                    .unwrap_or_default()
                    .to_duration()
                    .checked_sub(previous_comparison_time)
                    .unwrap_or_default()
                    .abs(), // Abs because later split might be shorter than previous
            ))
            .build();

        comparison_value.add_css_class("caption");
        comparison_value.add_css_class("timer");
        comparison_box.append(&comparison_label);
        comparison_box.append(&comparison_value);

        current_split.append(&best_box);
        current_split.append(&comparison_box);

        current_split
    }

    fn build_center_box_timer(timer: &Timer, config: &Config) -> GtkBox {
        // Right side: timer display
        let timer_box = GtkBox::new(Horizontal, 0);
        timer_box.add_css_class("timer");
        timer_box.add_css_class("greensplit");

        let formatted = format_timer(timer, config);
        let (left, right) = if let Some((l, r)) = formatted.rsplit_once('.') {
            (format!("{}.", l), r.to_string())
        } else {
            (formatted.clone(), String::new())
        };
        let hour_minutes_seconds_timer = Label::builder().label(left).build();
        hour_minutes_seconds_timer.add_css_class("bigtimer");

        let milis_timer = Label::builder().label(right).margin_top(14).build();
        milis_timer.add_css_class("smalltimer");

        timer_box.append(&hour_minutes_seconds_timer);
        timer_box.append(&milis_timer);

        timer_box
    }

    fn calculate_split_label_classes(
        timer: &Timer,
        comparison_duration: TimeDuration,
        split_duration: TimeDuration, // Either split duration or current attempt duration. Its meant to be the running duration of the split for the current attempt
        diff: TimeDuration,
        goldsplit_duration: TimeDuration,
        running: bool, // Serves to not show gold during running splits
    ) -> Vec<&'static str> {
        let mut classes = Vec::new();
        // First we check if its goldsplit. Priority over the rest
        if !running
            && goldsplit_duration != TimeDuration::ZERO
            && split_duration < goldsplit_duration
        {
            classes.push("goldsplit");
        } else if !running && goldsplit_duration == TimeDuration::ZERO {
            // Fisrt split
            classes.push("goldsplit");
            return classes;
        }

        // Now, we want to know wether we are ahead or behind comparison. (green or red)
        if diff.is_negative() {
            // Are we gaining or losing time?
            if split_duration <= comparison_duration {
                classes.push("greensplit");
            } else {
                classes.push("lostgreensplit");
            }
        } else if diff.is_positive() {
            // Are we gaining or losing time?
            if split_duration <= comparison_duration {
                classes.push("gainedredsplit");
            } else {
                classes.push("redsplit");
            }
        }

        classes
    }

    fn make_split_row(
        title: &str,
        value: &str,
        segment_classes: &Vec<&str>,
        label_classes: &Vec<&str>,
    ) -> adw::ActionRow {
        let row = adw::ActionRow::builder().title(title).build();
        let label = Label::builder()
            .label(value)
            .halign(Align::Center)
            .valign(Align::Center)
            .build();
        label.add_css_class("timer");
        for cls in segment_classes {
            row.add_css_class(cls);
        }
        for cls in label_classes {
            label.add_css_class(cls);
        }
        row.add_suffix(&label);

        row
    }
}
