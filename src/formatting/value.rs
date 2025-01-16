use std::borrow::Cow;
use std::time::Duration;

use super::Metadata;
use super::formatter;
use super::unit::Unit;
use chrono::{DateTime, Utc};
use chrono_tz::Tz;

#[derive(Debug, Clone)]
pub struct Value {
    pub inner: ValueInner,
    pub metadata: Metadata,
}

#[derive(Debug, Clone)]
pub enum BarGraphAccumulator {
    First,
    Last,
    Average,
}

#[derive(Debug, Clone)]
pub enum ValueInner {
    Text(String),
    Icon(Cow<'static, str>, Option<f64>),
    Number {
        val: f64,
        unit: Unit,
    },
    BarGraph {
        val: Vec<f64>,
        unit: Unit,
        accumulator: BarGraphAccumulator,
        min: Option<f64>,
        max: Option<f64>,
    },
    Datetime(DateTime<Utc>, Option<Tz>),
    Duration(Duration),
    Flag,
}

impl ValueInner {
    pub fn type_name(&self) -> &'static str {
        match self {
            ValueInner::Text(..) => "Text",
            ValueInner::Icon(..) => "Icon",
            ValueInner::Number { .. } => "Number",
            ValueInner::BarGraph { .. } => "bar_graph",
            ValueInner::Datetime(..) => "Datetime",
            ValueInner::Duration(..) => "Duration",
            ValueInner::Flag => "Flag",
        }
    }
}

pub trait IntoF64 {
    fn into_f64(self) -> f64;
}

macro_rules! impl_into_f64 {
    ($($t:ty),+) => {
        $(
            impl IntoF64 for $t {
                fn into_f64(self) -> f64 {
                    self as _
                }
            }
        )+
    }
}
impl_into_f64!(f64, f32, i64, u64, i32, u32, i16, u16, i8, u8, usize, isize);

/// Constructors
impl Value {
    pub fn new(val: ValueInner) -> Self {
        Self {
            inner: val,
            metadata: Default::default(),
        }
    }

    pub fn flag() -> Self {
        Self::new(ValueInner::Flag)
    }

    pub fn datetime(datetime: DateTime<Utc>, tz: Option<Tz>) -> Self {
        Self::new(ValueInner::Datetime(datetime, tz))
    }

    pub fn duration(duration: Duration) -> Self {
        Self::new(ValueInner::Duration(duration))
    }

    pub fn icon<S>(name: S) -> Self
    where
        S: Into<Cow<'static, str>>,
    {
        Self::new(ValueInner::Icon(name.into(), None))
    }

    pub fn icon_progression<S>(name: S, value: f64) -> Self
    where
        S: Into<Cow<'static, str>>,
    {
        Self::new(ValueInner::Icon(name.into(), Some(value)))
    }
    pub fn icon_progression_bound<S>(name: S, value: f64, low: f64, high: f64) -> Self
    where
        S: Into<Cow<'static, str>>,
    {
        Self::icon_progression(name, (value.clamp(low, high) - low) / (high - low))
    }

    pub fn text(text: String) -> Self {
        Self::new(ValueInner::Text(text))
    }

    pub fn number_unit(val: impl IntoF64, unit: Unit) -> Self {
        Self::new(ValueInner::Number {
            val: val.into_f64(),
            unit,
        })
    }

    pub fn bytes(val: impl IntoF64) -> Self {
        Self::number_unit(val, Unit::Bytes)
    }
    pub fn bits(val: impl IntoF64) -> Self {
        Self::number_unit(val, Unit::Bits)
    }
    pub fn percents(val: impl IntoF64) -> Self {
        Self::number_unit(val, Unit::Percents)
    }
    pub fn degrees(val: impl IntoF64) -> Self {
        Self::number_unit(val, Unit::Degrees)
    }
    pub fn seconds(val: impl IntoF64) -> Self {
        Self::number_unit(val, Unit::Seconds)
    }
    pub fn watts(val: impl IntoF64) -> Self {
        Self::number_unit(val, Unit::Watts)
    }
    pub fn hertz(val: impl IntoF64) -> Self {
        Self::number_unit(val, Unit::Hertz)
    }
    pub fn number(val: impl IntoF64) -> Self {
        Self::number_unit(val, Unit::None)
    }

    pub fn bar_graph_unit(
        val: &[impl IntoF64 + Copy],
        unit: Unit,
        accumulator: BarGraphAccumulator,
        min: Option<f64>,
        max: Option<f64>,
    ) -> Self {
        Self::new(ValueInner::BarGraph {
            val: val.iter().map(|v| v.into_f64()).collect(),
            unit,
            accumulator,
            min,
            max,
        })
    }

    pub fn bytes_bar_graph(
        val: &[impl IntoF64 + Copy],
        accumulator: BarGraphAccumulator,
        min: Option<f64>,
        max: Option<f64>,
    ) -> Self {
        Self::bar_graph_unit(val, Unit::Bytes, accumulator, min, max)
    }
    pub fn bits_bar_graph(
        val: &[impl IntoF64 + Copy],
        accumulator: BarGraphAccumulator,
        min: Option<f64>,
        max: Option<f64>,
    ) -> Self {
        Self::bar_graph_unit(val, Unit::Bits, accumulator, min, max)
    }
    pub fn percents_bar_graph(
        val: &[impl IntoF64 + Copy],
        accumulator: BarGraphAccumulator,
        min: Option<f64>,
        max: Option<f64>,
    ) -> Self {
        Self::bar_graph_unit(val, Unit::Percents, accumulator, min, max)
    }
    pub fn degrees_bar_graph(
        val: &[impl IntoF64 + Copy],
        accumulator: BarGraphAccumulator,
        min: Option<f64>,
        max: Option<f64>,
    ) -> Self {
        Self::bar_graph_unit(val, Unit::Degrees, accumulator, min, max)
    }
    pub fn seconds_bar_graph(
        val: &[impl IntoF64 + Copy],
        accumulator: BarGraphAccumulator,
        min: Option<f64>,
        max: Option<f64>,
    ) -> Self {
        Self::bar_graph_unit(val, Unit::Seconds, accumulator, min, max)
    }
    pub fn watts_bar_graph(
        val: &[impl IntoF64 + Copy],
        accumulator: BarGraphAccumulator,
        min: Option<f64>,
        max: Option<f64>,
    ) -> Self {
        Self::bar_graph_unit(val, Unit::Watts, accumulator, min, max)
    }
    pub fn hertz_bar_graph(
        val: &[impl IntoF64 + Copy],
        accumulator: BarGraphAccumulator,
        min: Option<f64>,
        max: Option<f64>,
    ) -> Self {
        Self::bar_graph_unit(val, Unit::Hertz, accumulator, min, max)
    }
    pub fn number_bar_graph(
        val: &[impl IntoF64 + Copy],
        accumulator: BarGraphAccumulator,
        min: Option<f64>,
        max: Option<f64>,
    ) -> Self {
        Self::bar_graph_unit(val, Unit::None, accumulator, min, max)
    }
}

/// Set options
impl Value {
    pub fn with_instance(mut self, instance: &'static str) -> Self {
        self.metadata.instance = Some(instance);
        self
    }

    pub fn underline(mut self, val: bool) -> Self {
        self.metadata.underline = val;
        self
    }

    pub fn italic(mut self, val: bool) -> Self {
        self.metadata.italic = val;
        self
    }

    pub fn default_formatter(&self) -> &'static dyn formatter::Formatter {
        match &self.inner {
            ValueInner::Text(_) | ValueInner::Icon(..) => &formatter::DEFAULT_STRING_FORMATTER,
            ValueInner::Number { .. } | ValueInner::BarGraph { .. } => {
                &formatter::DEFAULT_NUMBER_FORMATTER
            }
            ValueInner::Datetime { .. } => &*formatter::DEFAULT_DATETIME_FORMATTER,
            ValueInner::Duration { .. } => &formatter::DEFAULT_DURATION_FORMATTER,
            ValueInner::Flag => &formatter::DEFAULT_FLAG_FORMATTER,
        }
    }
}
