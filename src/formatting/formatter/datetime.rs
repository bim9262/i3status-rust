use std::sync::LazyLock;

use super::*;

const DEFAULT_DATETIME_FORMAT: &str = "%a %d/%m %R";

pub static DEFAULT_DATETIME_FORMATTER: LazyLock<DatetimeFormatter> =
    LazyLock::new(|| DatetimeFormatter::new(Some(DEFAULT_DATETIME_FORMAT), None, None).unwrap());

#[derive(Debug)]
pub enum DatetimeFormatter {
    Jiff {
        fmt: String,
        items: Vec<chrono::format::Item<'static>>,
        locale: Option<chrono::Locale>,
    },
    #[cfg(feature = "icu_calendar")]
    Icu {
        fieldset: icu_datetime::fieldsets::enums::CompositeDateTimeFieldSet,
        locale: icu_locale::Locale,
    },
}

impl DatetimeFormatter {
    pub(super) fn from_args(args: &[Arg]) -> Result<Self> {
        let mut format = None;
        let mut locale = None;
        let mut precision = None;
        for arg in args {
            match arg.key {
                "format" | "f" => {
                    format = Some(arg.val.error("format must be specified")?);
                }
                "locale" | "l" => {
                    locale = Some(arg.val.error("locale must be specified")?);
                }
                "precision" | "p" => {
                    precision = Some(arg.val.error("precision must be specified")?);
                }
                other => {
                    return Err(Error::new(format!(
                        "Unknown argument for 'datetime': '{other}'"
                    )));
                }
            }
        }
        Self::new(format, locale, precision)
    }

    fn new(format: Option<&str>, locale: Option<&str>, precision: Option<&str>) -> Result<Self> {
        let (items, locale) = match locale {
            Some(locale) => {
                #[cfg(feature = "icu_calendar")]
                let Ok(locale) = locale.try_into() else {
                    // try with icu4x
                    use icu_datetime::fieldsets::{
                        self,
                        enums::{CompositeDateTimeFieldSet, DateAndTimeFieldSet, DateFieldSet},
                    };
                    use icu_datetime::options::{Length, TimePrecision};
                    use std::str::FromStr as _;

                    let precision = match precision {
                        Some("seconds" | "second" | "s") => Some(TimePrecision::Second),
                        Some("minutes" | "minute" | "m") => Some(TimePrecision::Minute),
                        Some("hours" | "hour" | "h") => Some(TimePrecision::Hour),
                        None => None,
                        _ => Err(Error::new("Invalid precision value"))?,
                    };
                    let locale = icu_locale::Locale::from_str(locale)
                        .ok()
                        .error("invalid locale")?;
                    let fieldset = match format {
                        Some("full") => match precision {
                            Some(precision) => {
                                CompositeDateTimeFieldSet::DateTime(DateAndTimeFieldSet::YMDET(
                                    fieldsets::YMDET::long().with_time_precision(precision),
                                ))
                            }
                            None => CompositeDateTimeFieldSet::Date(DateFieldSet::YMDE(
                                fieldsets::YMDE::long(),
                            )),
                        },
                        length => {
                            let length = match length {
                                Some("short") => Length::Short,
                                Some("medium") => Length::Medium,
                                Some("long") | None => Length::Long,
                                _ => Err(Error::new("Invalid length value"))?,
                            };
                            match precision {
                                Some(precision) => {
                                    CompositeDateTimeFieldSet::DateTime(DateAndTimeFieldSet::YMDT(
                                        fieldsets::YMDT::for_length(length)
                                            .with_time_precision(precision),
                                    ))
                                }
                                None => CompositeDateTimeFieldSet::Date(DateFieldSet::YMD(
                                    fieldsets::YMD::for_length(length),
                                )),
                            }
                        }
                    };

                    return Ok(Self::Icu { locale, fieldset });
                };
                #[cfg(not(feature = "icu_calendar"))]
                let locale = locale.try_into().ok().error("invalid locale")?;
                if precision.is_some() {
                    return Err(Error::new(
                        "`precision` is only available for icu datetimes",
                    ));
                }
                (
                    chrono::format::StrftimeItems::new_with_locale(
                        format.unwrap_or(DEFAULT_DATETIME_FORMAT),
                        locale,
                    ),
                    Some(locale),
                )
            }
            None => {
                if precision.is_some() {
                    return Err(Error::new(
                        "`precision` is only available for icu datetimes",
                    ));
                }
                (
                    chrono::format::StrftimeItems::new(format.unwrap_or(DEFAULT_DATETIME_FORMAT)),
                    None,
                )
            }
        };

        Ok(Self::Jiff {
            fmt: format.unwrap_or(DEFAULT_DATETIME_FORMAT).to_string(),
            items: items.parse_to_owned().error(format!(
                "Invalid format: \"{}\"",
                format.unwrap_or(DEFAULT_DATETIME_FORMAT)
            ))?,
            locale,
        })
    }
}

impl Formatter for DatetimeFormatter {
    fn format(&self, val: &Value, _config: &SharedConfig) -> Result<String, FormatError> {
        match val {
            Value::Timestamp(timestamp, timezone) => {
                let zoned = timestamp
                    .to_owned()
                    .to_zoned(timezone.clone().unwrap_or_else(jiff::tz::TimeZone::system));
                match self {
                    DatetimeFormatter::Jiff { fmt, .. } => jiff::fmt::strtime::format(fmt, &zoned)
                        .map_err(|_| FormatError::IncompatibleFormatter {
                            ty: "Timestamp",
                            fmt: "datetime",
                        }),
                    #[cfg(feature = "icu_calendar")]
                    DatetimeFormatter::Icu {
                        locale,
                        fieldset: length,
                    } => {
                        use jiff_icu::ConvertFrom as _;
                        let dft = icu_datetime::DateTimeFormatter::try_new(locale.into(), *length)
                            .ok()
                            .error("locale should be present in compiled data")?;
                        Ok(dft
                            .format(&icu_datetime::input::DateTime::convert_from(
                                zoned.datetime(),
                            ))
                            .to_string())
                    }
                }
            }
            other => Err(FormatError::IncompatibleFormatter {
                ty: other.type_name(),
                fmt: "datetime",
            }),
        }
    }
}
