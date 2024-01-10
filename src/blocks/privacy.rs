//! Privacy Monitor
//!
//! # Configuration
//!
//! Key        | Values | Default|
//! -----------|--------|--------|
//! `driver` | The configuration of a driver (see below). | **Required**
//! `format`   | Format string. | <code>"{ $icon \|}"</code> |
//! `format_alt`   | Format string. | <code>"{ $icon $info \|}"</code> |
//!
//! # vl4 Options
//!
//! Key | Values | Required | Default
//! ----|--------|----------|--------
//! `name` | `vl4` | Yes | None
//! `exclude_device` | A device to ignore, example: `["/dev/video5"]` | No | `[]`
//! `exclude_consumer` | Processes to ignore | No | `["pipewire", "wireplumber"]`
//!
//! # Available Format Keys
//!
//! Placeholder   | Value                                          | Type     | Unit
//! --------------|------------------------------------------------|----------|-----
//! `icon`        | A static icon                                  | Icon     | -
//! `info`        | The mapping of which source are being consumed | Text     | -
//!
//! # Available Actions
//!
//! Action          | Description                               | Default button
//! ----------------|-------------------------------------------|---------------
//! `toggle_format` | Toggles between `format` and `format_alt` | Left
//!
//! # Example
//!
//! ```toml
//! [[block]]
//! block = "privacy"
//! [block.driver]
//! name = "v4l"
//! ```
//!
//! # Icons Used
//! - `webcam`

use super::prelude::*;

make_log_macro!(debug, "privacy");

mod v4l;

#[derive(Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct Config {
    #[serde(default)]
    pub format: FormatConfig,
    #[serde(default)]
    pub format_alt: FormatConfig,
    pub driver: PrivacyDriver,
}

#[derive(Deserialize, Debug)]
#[serde(tag = "name", rename_all = "snake_case")]
pub enum PrivacyDriver {
    V4l(v4l::Config),
}

// name -> [reader]
type PrivacyInfo = HashMap<String, Vec<String>>;

#[async_trait]
trait PrivacyMonitor {
    async fn get_info(&mut self) -> Result<PrivacyInfo>;
    async fn wait_for_change(&mut self) -> Result<()>;
    fn get_icon(&self) -> &'static str;
}

pub async fn run(config: &Config, api: &CommonApi) -> Result<()> {
    let mut actions = api.get_actions()?;
    api.set_default_actions(&[(MouseButton::Left, None, "toggle_format")])?;

    let mut format = config.format.with_default("{ $icon |}")?;
    let mut format_alt = config.format_alt.with_default("{ $icon $info |}")?;

    let mut device: Box<dyn PrivacyMonitor + Send + Sync> = match &config.driver {
        PrivacyDriver::V4l(driver_config) => {
            Box::new(v4l::Monitor::new(driver_config, api.error_interval).await?)
        }
    };

    loop {
        let mut widget = Widget::new().with_format(format.clone());

        let info = device.get_info().await?;

        if !info.is_empty() {
            widget.state = State::Warning;
            widget.set_values(map! {
                "icon" => Value::icon(device.get_icon()),
                "info" => Value::text(format!("{:?}", info)),
            });
        }

        api.set_widget(widget)?;

        select! {
            _ = api.wait_for_update_request() => (),
            _ = device.wait_for_change() =>(),
            Some(action) = actions.recv() => match action.as_ref() {
                "toggle_format" => {
                    std::mem::swap(&mut format_alt, &mut format);
                }
                _ => (),
            }
        }
    }
}
