"""Whether this pidgeon does its job: devices found and measurements pushed."""

from pidgeon_grafana.components.dashboard import dashboard
from pidgeon_grafana.components.field_helpers import (
  cell_options,
  field_property,
  override,
  pos,
  thresholds,
  value_mapping,
)
from pidgeon_grafana.components.grafana_enums import (
  CellBackgroundMode,
  CellDisplayMode,
  DrawStyle,
  FieldProperty,
  Matcher,
  StackingMode,
  TimeGroupFill,
  Unit,
)
from pidgeon_grafana.components.panels import (
  stat,
  state_timeline,
  table,
  timeseries,
)
from pidgeon_grafana.components.sql_helpers import time_group
from pidgeon_grafana.shared import StatusColor

# NOTE: values of the device_status enum in the migrations
DEVICE_STATUS = value_mapping(
  ("healthy", "Healthy", StatusColor.GOOD),
  ("unreachable", "Unreachable", StatusColor.WARNING),
  ("inactive", "Inactive", StatusColor.CRITICAL),
)


def device_count(status: str) -> str:
  return f"""
select count(*) as "{status}"
from devices
where status = '{status}'
"""


DASHBOARD = dashboard(
  "pidgeon-overview",
  "Overview",
  ("Is this pidgeon doing its job:"
   " discovered devices and pushing measurements to the cloud."),
  panels=[
    stat(
      "Healthy devices",
      device_count("healthy"),
      Unit.NONE,
      pos(0, 0, 4, 4),
    ),
    stat(
      "Unreachable devices",
      device_count("unreachable"),
      Unit.NONE,
      pos(4, 0, 4, 4),
      description="Ping failed but still within the inactive timeout.",
      steps=thresholds(
        StatusColor.GOOD,
        (1, StatusColor.WARNING),
      ),
    ),
    stat(
      "Inactive devices",
      device_count("inactive"),
      Unit.NONE,
      pos(8, 0, 4, 4),
      description=(
        "Not seen for longer than the inactive timeout; no longer measured."),
      steps=thresholds(
        StatusColor.GOOD,
        (1, StatusColor.CRITICAL),
      ),
    ),
    stat(
      "Last successful push",
      """
select extract(epoch from now() - max(timestamp)) as "Last successful push"
from logs
where kind = 'push' and status = 'success'
""",
      Unit.SECONDS,
      pos(12, 0, 6, 4),
      steps=thresholds(
        StatusColor.GOOD,
        (900, StatusColor.WARNING),
        (3600, StatusColor.CRITICAL),
      ),
      decimals=0,
    ),
    stat(
      "Unpushed measurements",
      """
select
  (select coalesce(max(id), 0) from measurements)
  - coalesce((
    select last
    from logs
    where kind = 'push' and status = 'success' and last is not null
    order by timestamp desc
    limit 1
  ), 0) as "Unpushed measurements"
""",
      Unit.SHORT,
      pos(18, 0, 6, 4),
      description="Measurements stored after the last successful push.",
    ),
    table(
      "Devices",
      """
select
  id as "Device",
  kind as "Kind",
  status::text as "Status",
  coalesce(host(address), path) as "Endpoint",
  slave as "Slave",
  seen as "Last seen",
  pinged as "Last pinged"
from devices
order by id
""",
      pos(0, 4, 24, 8),
      overrides=[
        override(
          Matcher.BY_NAME,
          "Status",
          field_property(
            FieldProperty.MAPPINGS,
            [DEVICE_STATUS],
          ),
          field_property(
            FieldProperty.CUSTOM_CELL_OPTIONS,
            cell_options(
              CellDisplayMode.COLOR_BACKGROUND,
              CellBackgroundMode.BASIC,
            ),
          ),
        ),
        override(
          Matcher.BY_REGEXP,
          "Last (seen|pinged)",
          field_property(
            FieldProperty.UNIT,
            Unit.DATE_TIME_FROM_NOW,
          ),
        ),
      ],
      sort_by="Device",
    ),
    state_timeline(
      "Device status",
      f"""
select
  {time_group()},
  source as metric,
  min(case status when 'healthy' then 2 when 'unreachable' then 1 else 0 end) as value
from health
where $__timeFilter(timestamp)
group by 1, source
order by 1
""",
      pos(0, 12, 24, 8),
      mappings=[
        value_mapping(
          (2, "Healthy", StatusColor.GOOD),
          (1, "Unreachable", StatusColor.WARNING),
          (0, "Inactive", StatusColor.CRITICAL),
        ),
      ],
      description="Worst status per interval, written by the ping process.",
    ),
    timeseries(
      "Pushes",
      f"""
select
  {time_group(TimeGroupFill.ZERO)},
  count(*) filter (where status = 'success') as "Succeeded",
  count(*) filter (where status = 'failure') as "Failed"
from logs
where $__timeFilter(timestamp) and kind = 'push'
group by 1
order by 1
""",
      Unit.SHORT,
      pos(0, 20, 12, 8),
      series=[
        ("Succeeded", StatusColor.GOOD),
        ("Failed", StatusColor.CRITICAL),
      ],
      draw_style=DrawStyle.BARS,
      stacking=StackingMode.NORMAL,
      min_=0,
      decimals=0,
      interval="1h",
    ),
    table(
      "Recent failures",
      """
select
  timestamp as "Time",
  kind::text as "Kind",
  left(response::text, 200) as "Response"
from logs
where $__timeFilter(timestamp) and status = 'failure'
order by timestamp desc
limit 50
""",
      pos(12, 20, 12, 8),
      overrides=[
        override(
          Matcher.BY_NAME,
          "Time",
          field_property(FieldProperty.CUSTOM_WIDTH, 180),
        ),
        override(
          Matcher.BY_NAME,
          "Kind",
          field_property(FieldProperty.CUSTOM_WIDTH, 80),
        ),
      ],
      inspect=True,
    ),
  ],
)
