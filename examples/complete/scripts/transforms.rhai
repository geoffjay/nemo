// Complete Example - Data Transform Functions

// Format the ticker value as an uptime string
fn format_uptime(value) {
    let seconds = value;
    if seconds == () { seconds = 0; }

    let hours = seconds / 3600;
    let minutes = (seconds % 3600) / 60;
    let secs = seconds % 60;

    let result = "";
    if hours > 0 {
        result += "" + hours + "h ";
    }
    if minutes > 0 || hours > 0 {
        result += "" + minutes + "m ";
    }
    result += "" + secs + "s";
    result
}

// Format a counter value with label
fn format_counter(value) {
    if value == () {
        return "Count: 0";
    }
    "Count: " + value
}

// Format stats refresh timestamp
fn format_timestamp(value) {
    if value == () {
        return "Last update: --";
    }
    "Last update: " + value + "s ago"
}
