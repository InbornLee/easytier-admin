/// 解析 easytier-cli 输出的人类可读大小，例如 "17.33kB" / "1.2MB" / "-"
pub fn parse_human_size(input: Option<&str>) -> f64 {
    let Some(text) = input else {
        return 0.0;
    };
    let text = text.trim();
    if text.is_empty() || text == "-" {
        return 0.0;
    }
    let mut num_end = 0usize;
    for (i, c) in text.char_indices() {
        if c.is_ascii_digit() || c == '.' || c == '-' || c == '+' {
            num_end = i + c.len_utf8();
        } else {
            break;
        }
    }
    let (num, rest) = text.split_at(num_end);
    let value: f64 = num.parse().unwrap_or(0.0);
    let unit = rest.trim().to_ascii_uppercase();
    let multiplier = match unit.as_str() {
        "" | "B" => 1.0,
        "KB" => 1000.0,
        "KIB" => 1024.0,
        "MB" => 1_000_000.0,
        "MIB" => 1_048_576.0,
        "GB" => 1_000_000_000.0,
        "GIB" => 1_073_741_824.0,
        "TB" => 1_000_000_000_000.0,
        "TIB" => 1_099_511_627_776.0,
        _ => 1.0,
    };
    value * multiplier
}

pub fn format_bytes(bytes: f64) -> String {
    if !bytes.is_finite() || bytes <= 0.0 {
        return "0 B".to_string();
    }
    let units = ["B", "KB", "MB", "GB", "TB"];
    let mut idx = 0usize;
    let mut value = bytes;
    while value >= 1024.0 && idx < units.len() - 1 {
        value /= 1024.0;
        idx += 1;
    }
    if value >= 100.0 || idx == 0 {
        format!("{value:.0} {}", units[idx])
    } else {
        format!("{value:.1} {}", units[idx])
    }
}

pub fn parse_int_safe(input: Option<&str>) -> i64 {
    input
        .and_then(|s| s.trim().parse::<i64>().ok())
        .unwrap_or(0)
}
