//! Example of a simple plugin using the nemo-plugin builder API
//!
//! This example shows how to create a basic plugin with a UI template
//! using the fluent builder pattern.

use nemo_plugin::containers;
use nemo_plugin::prelude::*;

/// Build a simple temperature monitor UI
fn build_temperature_monitor() -> PluginValue {
    Panel::new()
        .padding(20)
        .border(1)
        .border_color("theme.border")
        .shadow("md")
        .width(350)
        .child(
            "header",
            Stack::vertical()
                .spacing(4)
                .child(
                    "title",
                    Label::new("Temperature Monitor").size("xl").weight("bold"),
                )
                .child(
                    "subtitle",
                    Label::new("Real-time sensor readings")
                        .size("sm")
                        .color("theme.muted"),
                ),
        )
        .child(
            "controls",
            Stack::vertical()
                .spacing(16)
                // Temperature display
                .child(
                    "temp_display",
                    Panel::new()
                        .padding(16)
                        .bg_color("theme.surface")
                        .border(1)
                        .child(
                            "content",
                            Stack::vertical()
                                .spacing(8)
                                .align("center")
                                .child("label", Label::new("Current Temperature"))
                                .child(
                                    "value",
                                    Label::new("--°C")
                                        .size("xl")
                                        .weight("bold")
                                        .bind_text("data.temperature.display"),
                                ),
                        ),
                )
                // Settings
                .child(
                    "settings",
                    Stack::vertical()
                        .spacing(12)
                        // Temperature unit selector
                        .child(
                            "unit_row",
                            Stack::horizontal()
                                .spacing(8)
                                .child("label", Label::new("Unit:").width(100))
                                .child(
                                    "celsius_btn",
                                    Button::new("°C")
                                        .variant("secondary")
                                        .on_click("set_celsius")
                                        .width(60),
                                )
                                .child(
                                    "fahrenheit_btn",
                                    Button::new("°F")
                                        .variant("secondary")
                                        .on_click("set_fahrenheit")
                                        .width(60),
                                ),
                        )
                        // Alert threshold
                        .child(
                            "threshold_row",
                            Stack::vertical()
                                .spacing(8)
                                .child("threshold_label", Label::new("Alert Threshold"))
                                .child(
                                    "threshold_slider",
                                    Slider::new()
                                        .min(0.0)
                                        .max(100.0)
                                        .step(1.0)
                                        .value(30.0)
                                        .on_change("on_threshold_change")
                                        .bind_value("settings.alert_threshold"),
                                ),
                        )
                        // Enable alerts switch
                        .child(
                            "alerts_switch",
                            Switch::new()
                                .label("Enable High Temperature Alerts")
                                .checked(true)
                                .on_click("toggle_alerts")
                                .bind_checked("settings.alerts_enabled"),
                        ),
                )
                // History chart placeholder
                .child(
                    "history",
                    Panel::new()
                        .padding(12)
                        .bg_color("theme.surface")
                        .border(1)
                        .height(200)
                        .child(
                            "chart_label",
                            Label::new("Temperature History").weight("bold"),
                        ),
                ),
        )
        .build()
}

/// Build a control panel with multiple inputs
fn build_control_panel() -> PluginValue {
    Panel::new()
        .padding(16)
        .border(2)
        .border_color("theme.primary")
        .child(
            "form",
            Stack::vertical()
                .spacing(12)
                .child("title", Label::new("Device Control Panel").size("lg"))
                // Using the helper function for input rows
                .child(
                    "name_row",
                    containers::input_row(
                        "Device Name",
                        Some(120),
                        Input::new()
                            .placeholder("Enter device name")
                            .on_change("on_name_change"),
                        Some(8),
                    ),
                )
                .child(
                    "id_row",
                    containers::input_row(
                        "Device ID",
                        Some(120),
                        Input::new()
                            .placeholder("Auto-generated")
                            .disabled(true)
                            .bind_value("device.id"),
                        Some(8),
                    ),
                )
                // Action buttons
                .child(
                    "actions",
                    Stack::horizontal()
                        .spacing(8)
                        .justify("end")
                        .child(
                            "save_btn",
                            Button::new("Save")
                                .variant("primary")
                                .on_click("save_settings"),
                        )
                        .child(
                            "reset_btn",
                            Button::new("Reset")
                                .variant("secondary")
                                .on_click("reset_settings"),
                        )
                        .child(
                            "delete_btn",
                            Button::new("Delete")
                                .variant("danger")
                                .on_click("delete_device"),
                        ),
                ),
        )
        .build()
}

/// Build a dashboard with grid layout
fn build_dashboard() -> PluginValue {
    Panel::new()
        .padding(20)
        .child(
            "dashboard",
            Stack::vertical()
                .spacing(16)
                .child(
                    "header",
                    Label::new("System Dashboard").size("xl").weight("bold"),
                )
                .child(
                    "metrics",
                    Grid::new(2)
                        .gap(16)
                        // Metric card 1
                        .child(
                            "cpu",
                            Panel::new().padding(16).border(1).child(
                                "content",
                                Stack::vertical()
                                    .spacing(8)
                                    .child("label", Label::new("CPU Usage"))
                                    .child(
                                        "value",
                                        Label::new("--%")
                                            .size("xl")
                                            .weight("bold")
                                            .bind_text("metrics.cpu.display"),
                                    )
                                    .child(
                                        "bar",
                                        Slider::new()
                                            .min(0.0)
                                            .max(100.0)
                                            .bind_value("metrics.cpu.value"),
                                    ),
                            ),
                        )
                        // Metric card 2
                        .child(
                            "memory",
                            Panel::new().padding(16).border(1).child(
                                "content",
                                Stack::vertical()
                                    .spacing(8)
                                    .child("label", Label::new("Memory Usage"))
                                    .child(
                                        "value",
                                        Label::new("--%")
                                            .size("xl")
                                            .weight("bold")
                                            .bind_text("metrics.memory.display"),
                                    )
                                    .child(
                                        "bar",
                                        Slider::new()
                                            .min(0.0)
                                            .max(100.0)
                                            .bind_value("metrics.memory.value"),
                                    ),
                            ),
                        )
                        // Metric card 3
                        .child(
                            "disk",
                            Panel::new().padding(16).border(1).child(
                                "content",
                                Stack::vertical()
                                    .spacing(8)
                                    .child("label", Label::new("Disk Usage"))
                                    .child(
                                        "value",
                                        Label::new("--%")
                                            .size("xl")
                                            .weight("bold")
                                            .bind_text("metrics.disk.display"),
                                    )
                                    .child(
                                        "bar",
                                        Slider::new()
                                            .min(0.0)
                                            .max(100.0)
                                            .bind_value("metrics.disk.value"),
                                    ),
                            ),
                        )
                        // Metric card 4
                        .child(
                            "network",
                            Panel::new().padding(16).border(1).child(
                                "content",
                                Stack::vertical()
                                    .spacing(8)
                                    .child("label", Label::new("Network"))
                                    .child(
                                        "value",
                                        Label::new("-- MB/s")
                                            .size("xl")
                                            .weight("bold")
                                            .bind_text("metrics.network.display"),
                                    ),
                            ),
                        ),
                ),
        )
        .build()
}

fn main() {
    println!("Building temperature monitor...");
    let temp_monitor = build_temperature_monitor();
    println!("Temperature monitor: {:#?}", temp_monitor);

    println!("\nBuilding control panel...");
    let control_panel = build_control_panel();
    println!("Control panel: {:#?}", control_panel);

    println!("\nBuilding dashboard...");
    let dashboard = build_dashboard();
    println!("Dashboard: {:#?}", dashboard);
}
