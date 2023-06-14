pub const SED_STRIP_ANSI_CODES: &str = r"sed 's/\x1b\[[0-9;]*m//g'";

pub fn awk_indent_lines(indent: &str) -> String {
    format!(r#"awk -v indent={:?} '{{print indent $0}}'"#, indent)
}

pub fn awk_chop_lines(width: usize) -> String {
    format!(r#"awk -v width={} '{{print substr($0, 1, width)}}'"#, width)
}

pub fn notify_send_critical(subject: &str, body: &str) -> String {
    format!("notify-send -u critical {} {}", subject, body)
}

pub fn tail_log(
    file: &str,
    lines: usize,
    indent: &str,
    width_limit: usize,
) -> String {
    format!(
        "tail -n {} {} | {} | {} | {}",
        lines,
        file,
        SED_STRIP_ANSI_CODES,
        awk_indent_lines(indent),
        awk_chop_lines(width_limit)
    )
}

#[cfg(test)]
mod tests {
    #[test]
    fn awk_indent_lines() {
        assert_eq!(
            "awk -v indent=\"_\" '{print indent $0}'",
            super::awk_indent_lines("_")
        )
    }

    #[test]
    fn awk_chop_lines() {
        assert_eq!(
            "awk -v width=5 '{print substr($0, 1, width)}'",
            super::awk_chop_lines(5)
        )
    }

    #[test]
    fn tail_log() {
        assert_eq!(
            r#"tail -n 5 ./file | sed 's/\x1b\[[0-9;]*m//g' | awk -v indent="_" '{print indent $0}' | awk -v width=5 '{print substr($0, 1, width)}'"#,
            super::tail_log("./file", 5, "_", 5)
        )
    }

    #[test]
    fn notify_simple() {
        assert_eq!(
            "notify-send -u critical subject body",
            super::notify_send_critical("subject", "body")
        )
    }

    #[test]
    fn notify_compound_no_quotes() {
        assert_eq!(
            r#"notify-send -u critical subject $(tail -n 5 ./file | sed 's/\x1b\[[0-9;]*m//g' | awk -v indent="_" '{print indent $0}' | awk -v width=5 '{print substr($0, 1, width)}')"#,
            super::notify_send_critical(
                "subject",
                &format!("$({})", super::tail_log("./file", 5, "_", 5))
            )
        )
    }

    #[test]
    fn notify_compound_with_quotes() {
        assert_eq!(
            r#"notify-send -u critical 'foo bar baz' $(tail -n 5 "./file" | sed 's/\x1b\[[0-9;]*m//g' | awk -v indent="_" '{print indent $0}' | awk -v width=5 '{print substr($0, 1, width)}')"#,
            super::notify_send_critical(
                "'foo bar baz'",
                &format!(
                    "$({})",
                    super::tail_log(&format!("{:?}", "./file"), 5, "_", 5)
                )
            )
        )
    }
}
