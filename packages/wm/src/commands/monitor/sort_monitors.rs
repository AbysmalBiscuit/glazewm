use std::cmp::Ordering;

use wm_common::MonitorConfig;
use wm_platform::Rect;

use crate::{
  models::{Monitor, RootContainer},
  traits::{CommonGetters, PositionGetters},
};

/// Sorts the root container's monitors by pinned config position, then
/// from left-to-right and top-to-bottom.
///
/// Monitors matching an entry in the `monitors` config are ordered by
/// that entry's position in the config list. Monitors without a match
/// are ordered after all matched ones, by physical position.
pub fn sort_monitors(
  root: &RootContainer,
  monitor_configs: &[MonitorConfig],
) -> anyhow::Result<()> {
  let monitors = root.monitors();

  warn_unmatched_configs(&monitors, monitor_configs);

  // Create a tuple of monitors and their sort keys.
  let mut monitors_with_keys = monitors
    .into_iter()
    .map(|monitor| {
      let rect = monitor.to_rect()?;
      let config_position = config_position(&monitor, monitor_configs);
      anyhow::Ok((monitor, config_position, rect))
    })
    .try_collect::<Vec<_>>()?;

  monitors_with_keys.sort_by(|(_, pos_a, rect_a), (_, pos_b, rect_b)| {
    compare_monitors(*pos_a, rect_a, *pos_b, rect_b)
  });

  *root.borrow_children_mut() = monitors_with_keys
    .into_iter()
    .map(|(monitor, _, _)| monitor.into())
    .collect();

  Ok(())
}

/// Gets the position of the monitor's machine ID within the `monitors`
/// config, if any entry matches.
///
/// # Platform-specific
///
/// - Always `None` on macOS; the `monitors` config is Windows-only.
fn config_position(
  monitor: &Monitor,
  monitor_configs: &[MonitorConfig],
) -> Option<usize> {
  #[cfg(target_os = "windows")]
  {
    let machine_id = monitor.native_properties().machine_id()?;

    monitor_configs
      .iter()
      .position(|config| config.machine_id == machine_id)
  }

  #[cfg(not(target_os = "windows"))]
  {
    let _ = (monitor, monitor_configs);
    None
  }
}

/// Compares two monitors by pinned config position, falling back to
/// physical position (left-to-right, top-to-bottom) when neither is
/// pinned.
fn compare_monitors(
  config_pos_a: Option<usize>,
  rect_a: &Rect,
  config_pos_b: Option<usize>,
  rect_b: &Rect,
) -> Ordering {
  match (config_pos_a, config_pos_b) {
    (Some(pos_a), Some(pos_b)) => pos_a.cmp(&pos_b),
    (Some(_), None) => Ordering::Less,
    (None, Some(_)) => Ordering::Greater,
    (None, None) => {
      if rect_a.x() == rect_b.x() {
        rect_a.y().cmp(&rect_b.y())
      } else {
        rect_a.x().cmp(&rect_b.x())
      }
    }
  }
}

/// Warns about `monitors` config entries that match none of the
/// connected monitors (e.g. typos or unplugged monitors).
///
/// # Platform-specific
///
/// - On macOS, warns that the `monitors` config is unsupported.
fn warn_unmatched_configs(
  monitors: &[Monitor],
  monitor_configs: &[MonitorConfig],
) {
  #[cfg(target_os = "windows")]
  {
    let machine_ids = monitors
      .iter()
      .filter_map(|monitor| monitor.native_properties().machine_id())
      .collect::<Vec<_>>();

    let unmatched = monitor_configs
      .iter()
      .filter(|config| !machine_ids.contains(&config.machine_id))
      .map(|config| config.machine_id.as_str())
      .collect::<Vec<_>>();

    if !unmatched.is_empty() {
      tracing::warn!(
        "No connected monitor matches machine ID(s) {unmatched:?} from \
         the `monitors` config. Connected machine IDs: {machine_ids:?}.",
      );
    }
  }

  #[cfg(not(target_os = "windows"))]
  {
    let _ = monitors;

    if !monitor_configs.is_empty() {
      tracing::warn!(
        "The `monitors` config is only supported on Windows."
      );
    }
  }
}

#[cfg(test)]
mod tests {
  use std::cmp::Ordering;

  use wm_platform::Rect;

  use super::compare_monitors;

  fn rect_at(x: i32, y: i32) -> Rect {
    Rect::from_xy(x, y, 1920, 1080)
  }

  #[test]
  fn pinned_monitors_sort_by_config_position() {
    // Config position wins even when physical position disagrees.
    assert_eq!(
      compare_monitors(
        Some(0),
        &rect_at(5000, 0),
        Some(1),
        &rect_at(0, 0)
      ),
      Ordering::Less
    );
  }

  #[test]
  fn config_position_gaps_preserve_relative_order() {
    // A disconnected entry between two connected ones (positions 1 and
    // 3) leaves their relative order intact; indices compact naturally
    // since only connected monitors are sorted.
    assert_eq!(
      compare_monitors(
        Some(1),
        &rect_at(5000, 0),
        Some(3),
        &rect_at(0, 0)
      ),
      Ordering::Less
    );
  }

  #[test]
  fn pinned_monitor_sorts_before_unpinned() {
    assert_eq!(
      compare_monitors(Some(3), &rect_at(5000, 0), None, &rect_at(0, 0)),
      Ordering::Less
    );
    assert_eq!(
      compare_monitors(None, &rect_at(0, 0), Some(3), &rect_at(5000, 0)),
      Ordering::Greater
    );
  }

  #[test]
  fn unpinned_monitors_sort_by_physical_position() {
    // Left-to-right.
    assert_eq!(
      compare_monitors(None, &rect_at(0, 0), None, &rect_at(1920, 0)),
      Ordering::Less
    );

    // Top-to-bottom at equal x.
    assert_eq!(
      compare_monitors(None, &rect_at(0, 0), None, &rect_at(0, 1080)),
      Ordering::Less
    );
  }
}
