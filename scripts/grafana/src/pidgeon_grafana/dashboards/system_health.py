from pidgeon_grafana.components.dashboard import annotation, dashboard
from pidgeon_grafana.components.field_helpers import (
  pos,
  thresholds,
  value_mapping,
)
from pidgeon_grafana.components.grafana_enums import (
  QueryFormat,
  StatGraphMode,
  Unit,
)
from pidgeon_grafana.components.panels import (
  stat,
  state_timeline,
  timeseries,
)
from pidgeon_grafana.components.sql_helpers import (
  average_per_interval,
  latest,
  time_group,
)
from pidgeon_grafana.shared import SeriesColor, StatusColor

TABLE = "system_health"


def averages(*columns: tuple[str, str]) -> str:
  return average_per_interval(TABLE, columns)


def newest(expression: str, name: str) -> str:
  return latest(TABLE, expression, name)


DASHBOARD = dashboard(
  "pidgeon-system-health",
  "System health",
  ("Raspberry Pi health from the system_health table"
   " (one row per minute, kept for one month)."),
  panels=[
    stat(
      "Uptime",
      newest("uptime_seconds", "Uptime"),
      Unit.DURATION_SECONDS,
      pos(0, 0, 4, 4),
    ),
    stat(
      "CPU temperature",
      newest("cpu_temperature_celsius", "CPU temperature"),
      Unit.CELSIUS,
      pos(4, 0, 4, 4),
      description="Raspberry Pi 4 starts throttling at 80 °C.",
      steps=thresholds(
        StatusColor.GOOD,
        (70, StatusColor.WARNING),
        (80, StatusColor.CRITICAL),
      ),
    ),
    stat(
      "Memory available",
      newest(
        "100.0 * memory_available_bytes / nullif(memory_total_bytes, 0)",
        "Memory available",
      ),
      Unit.PERCENT,
      pos(8, 0, 4, 4),
      steps=thresholds(
        StatusColor.CRITICAL,
        (10, StatusColor.WARNING),
        (20, StatusColor.GOOD),
      ),
      decimals=0,
    ),
    stat(
      "Throttling",
      newest(
        "case when throttled_flags is null then null"
        " when (throttled_flags & 15) <> 0 then 2"
        " when throttled_flags <> 0 then 1 else 0 end",
        "Throttling",
      ),
      Unit.NONE,
      pos(12, 0, 4, 4),
      description=("From the firmware throttled flags: low bits are active now,"
                   " high bits mean it happened since boot."),
      mappings=[
        value_mapping(
          (0, "None", StatusColor.GOOD),
          (1, "Since boot", StatusColor.WARNING),
          (2, "Active", StatusColor.CRITICAL),
        ),
      ],
    ),
    stat(
      "Pidgeon memory",
      averages(("Pidgeon memory", "pidgeon_rss_bytes")),
      Unit.BYTES,
      pos(16, 0, 4, 4),
      fmt=QueryFormat.TIME_SERIES,
      graph_mode=StatGraphMode.AREA,
    ),
    stat(
      "Database size",
      averages(("Database size", "database_bytes")),
      Unit.BYTES,
      pos(20, 0, 4, 4),
      fmt=QueryFormat.TIME_SERIES,
      graph_mode=StatGraphMode.AREA,
    ),
    timeseries(
      "CPU temperature",
      averages(("CPU", "cpu_temperature_celsius")),
      Unit.CELSIUS,
      pos(0, 4, 12, 8),
    ),
    timeseries(
      "CPU frequency",
      averages(("CPU", "cpu_frequency_mhz * 1e6")),
      Unit.HERTZ,
      pos(12, 4, 12, 8),
      min_=0,
    ),
    timeseries(
      "CPU usage",
      averages(
        ("Usage", "cpu_usage_percent"),
        ("IO wait", "cpu_iowait_percent"),
      ),
      Unit.PERCENT,
      pos(0, 12, 12, 8),
      series=[
        ("Usage", SeriesColor.S1),
        ("IO wait", SeriesColor.S2),
      ],
      min_=0,
      max_=100,
    ),
    timeseries(
      "Load average",
      averages(
        ("1m", "load_1m"),
        ("5m", "load_5m"),
        ("15m", "load_15m"),
      ),
      Unit.SHORT,
      pos(12, 12, 12, 8),
      series=[
        ("1m", SeriesColor.S1),
        ("5m", SeriesColor.S2),
        ("15m", SeriesColor.S3),
      ],
      description=("Raspberry Pi 4 has 4 cores;"
                   " sustained load above 4 means work is queueing."),
      min_=0,
    ),
    timeseries(
      "Memory",
      averages(
        ("Used", "memory_total_bytes - memory_available_bytes"),
        ("Swap", "swap_used_bytes"),
      ),
      Unit.BYTES,
      pos(0, 20, 8, 8),
      series=[
        ("Used", SeriesColor.S1),
        ("Swap", SeriesColor.S2),
      ],
      min_=0,
    ),
    timeseries(
      "Pidgeon memory",
      averages(("Pidgeon", "pidgeon_rss_bytes")),
      Unit.BYTES,
      pos(8, 20, 8, 8),
      description=("Resident memory of the pidgeon process;"
                   " steady growth between reboots points to a leak."),
    ),
    timeseries(
      "Database size",
      averages(("Database", "database_bytes")),
      Unit.BYTES,
      pos(16, 20, 8, 8),
      description="Measurements are never deleted, so this only grows.",
    ),
    state_timeline(
      "Throttling",
      f"""
select
  {time_group()},
  max(throttled_flags & 1) as "Under-voltage",
  max((throttled_flags >> 1) & 1) as "Frequency capped",
  max((throttled_flags >> 2) & 1) as "Throttled",
  max((throttled_flags >> 3) & 1) as "Soft temperature limit"
from system_health
where $__timeFilter(timestamp) and throttled_flags is not null
group by 1
order by 1
""",
      pos(0, 28, 24, 6),
      mappings=[
        value_mapping(
          (0, "OK", StatusColor.TRANSPARENT),
          (1, "Active", StatusColor.CRITICAL),
        ),
      ],
      description=(
        "Raspberry Pi firmware throttled flags (bits 0-3). Empty in a VM."),
    ),
  ],
  annotations=[
    annotation(
      "Reboots",
      """
select boot as time, 'Boot ' || left(boot_id, 8) as text
from (
  select boot_id, min(timestamp - uptime_seconds * interval '1 second') as boot
  from system_health
  group by boot_id
) boots
where $__timeFilter(boot)
order by 1
""",
      StatusColor.NEUTRAL,
    ),
  ],
)
