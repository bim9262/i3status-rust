use crate::util;

use super::*;

const DEFAULT_BAR_WIDTH_HORIZONTAL: usize = 5;

#[derive(Debug)]
pub struct BarGraphFormatter {
    width: usize,
}

impl BarGraphFormatter {
    pub(super) fn from_args(args: &[Arg]) -> Result<Self> {
        let mut width = DEFAULT_BAR_WIDTH_HORIZONTAL;
        for arg in args {
            match arg.key {
                "width" | "w" => {
                    width = arg
                        .val
                        .error("width must be specified")?
                        .parse()
                        .error("width must be a positive integer")?;
                }

                other => {
                    return Err(Error::new(format!("Unknown argument for 'bar': '{other}'")));
                }
            }
        }
        Ok(Self { width })
    }
}

impl Formatter for BarGraphFormatter {
    fn format(&self, val: &Value, _config: &SharedConfig) -> Result<String, FormatError> {
        match val {
            Value::Number { .. } => Ok("".to_string()),
            Value::BarGraph { val, min, max, .. } => Ok(util::format_bar_graph(val, *min, *max)),
            other => Err(FormatError::IncompatibleFormatter {
                ty: other.type_name(),
                fmt: "bar_graph",
            }),
        }
    }

    fn data_points(&self) -> Option<usize> {
        Some(self.width)
    }
}
