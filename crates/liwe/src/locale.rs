use chrono::Locale;
use std::str::FromStr;

pub fn get_locale(config_locale: Option<&str>) -> Locale {
    config_locale
        .map(String::from)
        .or_else(sys_locale::get_locale)
        .and_then(|s| parse_locale(&s))
        .unwrap_or(Locale::POSIX)
}

fn parse_locale(locale_str: &str) -> Option<Locale> {
    let code = locale_str.replace('-', "_");
    let code = code.split('.').next().unwrap_or(&code);
    Locale::from_str(code).ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    #[test]
    fn test_config_locale_override() {
        let locale = get_locale(Some("de_DE"));
        assert_eq!(locale, Locale::de_DE);
    }

    #[test]
    fn test_bcp47_format_conversion() {
        let locale = get_locale(Some("de-DE"));
        assert_eq!(locale, Locale::de_DE);
    }

    #[test]
    fn test_encoding_suffix_stripped() {
        let locale = get_locale(Some("de_DE.UTF-8"));
        assert_eq!(locale, Locale::de_DE);
    }

    #[test]
    fn test_invalid_locale_falls_back_to_posix() {
        let locale = get_locale(Some("invalid_LOCALE"));
        assert_eq!(locale, Locale::POSIX);
    }

    #[test]
    fn test_german_date_formatting() {
        let date = NaiveDate::from_ymd_opt(2026, 3, 27).unwrap();
        let locale = get_locale(Some("de_DE"));
        let formatted = date.format_localized("%A, %d. %B %Y", locale).to_string();
        assert_eq!(formatted, "Freitag, 27. März 2026");
    }

    #[test]
    fn test_datetime_with_time_components() {
        use chrono::{FixedOffset, TimeZone};

        let tz = FixedOffset::east_opt(0).unwrap();
        let dt = tz.with_ymd_and_hms(2026, 3, 27, 14, 30, 45).unwrap();
        let locale = get_locale(None);

        assert_eq!(
            dt.format_localized("%Y-%m-%d", locale).to_string(),
            "2026-03-27"
        );
        assert_eq!(
            dt.format_localized("%H:%M:%S", locale).to_string(),
            "14:30:45"
        );
        assert_eq!(
            dt.format_localized("%Y%m%d%H%M", locale).to_string(),
            "202603271430"
        );
    }
}
