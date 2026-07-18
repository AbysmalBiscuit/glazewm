use wm_platform::{Display, Rect};
#[cfg(target_os = "windows")]
use wm_platform::{DisplayDeviceExtWindows, DisplayExtWindows};

#[derive(Debug, Clone)]
pub struct NativeMonitorProperties {
  #[cfg(target_os = "macos")]
  pub device_uuid: String,
  #[cfg(target_os = "windows")]
  pub handle: isize,
  #[cfg(target_os = "windows")]
  pub hardware_id: Option<String>,
  #[cfg(target_os = "windows")]
  pub device_path: Option<String>,
  pub device_name: String,
  pub working_area: Rect,
  pub bounds: Rect,
  pub dpi: u32,
  pub scale_factor: f32,
}

impl NativeMonitorProperties {
  pub fn try_from(native_display: &Display) -> anyhow::Result<Self> {
    let display_device = native_display.main_device()?;

    Ok(Self {
      #[cfg(target_os = "macos")]
      device_uuid: display_device.id().0,
      #[cfg(target_os = "windows")]
      handle: native_display.hmonitor().0,
      #[cfg(target_os = "windows")]
      hardware_id: display_device.hardware_id(),
      #[cfg(target_os = "windows")]
      device_path: display_device.device_path(),
      device_name: native_display.name()?,
      working_area: native_display.working_area()?,
      bounds: native_display.bounds()?,
      dpi: native_display.dpi()?,
      scale_factor: native_display.scale_factor()?,
    })
  }

  /// Extracts a stable machine ID from the monitor's device path.
  ///
  /// Strips the `\\?\DISPLAY#` prefix and the trailing `#{guid}` segment
  /// from device paths like
  /// `\\?\DISPLAY#MSI3CA8#5&2fdae59&0&UID4358#{e6f07b5f-...}` to produce
  /// `MSI3CA8#5&2fdae59&0&UID4358`.
  ///
  /// Returns `None` for displays without a device path (e.g. virtual
  /// displays) or with an unexpected format.
  ///
  /// # Platform-specific
  ///
  /// - This method is only available on Windows.
  #[cfg(target_os = "windows")]
  pub fn machine_id(&self) -> Option<String> {
    let device_path = self.device_path.as_deref()?;
    let stripped = device_path.strip_prefix(r"\\?\DISPLAY#")?;
    let (machine_id, _) = stripped.split_once("#{")?;

    Some(machine_id.to_string())
  }
}

#[cfg(all(test, target_os = "windows"))]
mod tests {
  use super::*;

  fn props_with_path(
    device_path: Option<&str>,
  ) -> NativeMonitorProperties {
    NativeMonitorProperties {
      handle: 0,
      hardware_id: None,
      device_path: device_path.map(ToString::to_string),
      device_name: String::new(),
      working_area: Rect::from_xy(0, 0, 1920, 1080),
      bounds: Rect::from_xy(0, 0, 1920, 1080),
      dpi: 96,
      scale_factor: 1.0,
    }
  }

  #[test]
  fn machine_id_from_device_path() {
    let props = props_with_path(Some(
      r"\\?\DISPLAY#MSI3CA8#5&2fdae59&0&UID4358#{e6f07b5f-ee97-4a90-b076-33f57bf4eaa7}",
    ));

    assert_eq!(
      props.machine_id().as_deref(),
      Some("MSI3CA8#5&2fdae59&0&UID4358")
    );
  }

  #[test]
  fn machine_id_without_device_path() {
    assert_eq!(props_with_path(None).machine_id(), None);
  }

  #[test]
  fn machine_id_with_malformed_device_path() {
    // Missing the `\\?\DISPLAY#` prefix.
    assert_eq!(
      props_with_path(Some("MSI3CA8#5&2fdae59&0&UID4358")).machine_id(),
      None
    );

    // Missing the `#{guid}` suffix.
    assert_eq!(
      props_with_path(Some(r"\\?\DISPLAY#MSI3CA8#5&2fdae59&0&UID4358"))
        .machine_id(),
      None
    );
  }
}
