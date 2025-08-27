// SPDX-FileCopyrightText: 2025 Ryan Cao <hello@ryanccn.dev>
//
// SPDX-License-Identifier: GPL-3.0-or-later

use owo_colors::OwoColorize as _;

const HOUR: i32 = 60 * 60;
const MINUTE: i32 = 60;

pub fn format_duration(duration_secs: i32, cyan: bool) -> String {
    let mut duration_secs = duration_secs;
    let mut str = String::new();
    let mut has_started = false;

    if has_started || duration_secs >= HOUR {
        let hours = duration_secs / HOUR;

        if cyan {
            str = format!("{}{:.0}{}", str, hours, "h".dimmed());
        } else {
            str = format!("{}{:.0}{}", str, hours.cyan(), "h".cyan().dimmed());
        }

        duration_secs -= hours * HOUR;
        has_started = true;
    }

    if has_started || duration_secs >= MINUTE {
        let mins = duration_secs / MINUTE;

        if cyan {
            str = format!("{}{:.0}{}", str, mins.cyan(), "m".cyan().dimmed());
        } else {
            str = format!("{}{:.0}{}", str, mins, "m".dimmed());
        }

        duration_secs -= mins * MINUTE;
        // has_started = true;
    }

    if cyan {
        str = format!("{}{:.0}{}", str, duration_secs.cyan(), "s".cyan().dimmed());
    } else {
        str = format!("{}{:.0}{}", str, duration_secs, "s".dimmed());
    }

    str
}

pub fn format_duration_plain(duration_secs: i32) -> String {
    let mut duration_secs = duration_secs;
    let mut str = String::new();
    let mut has_started = false;

    if has_started || duration_secs > HOUR {
        let hours = duration_secs / HOUR;

        str = format!("{}{:.0}{}", str, hours, "h");
        duration_secs -= hours * HOUR;
        has_started = true;
    }

    if has_started || duration_secs > MINUTE {
        let mins = duration_secs / MINUTE;

        str = format!("{}{:.0}{}", str, mins, "m");
        duration_secs -= mins * MINUTE;
        // has_started = true;
    }

    str = format!("{}{:.0}{}", str, duration_secs, "s");
    str
}
