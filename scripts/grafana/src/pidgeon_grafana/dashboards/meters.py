from pidgeon_grafana.components.dashboard import dashboard, query_variable
from pidgeon_grafana.components.field_helpers import pos, thresholds
from pidgeon_grafana.components.grafana_enums import (
  DrawStyle,
  StackingMode,
  Unit,
)
from pidgeon_grafana.components.panels import row, stat, timeseries
from pidgeon_grafana.components.sql_helpers import (
  average_per_interval,
  json_field,
  latest,
)
from pidgeon_grafana.shared import TIMEZONE, SeriesColor, StatusColor

TABLE = "measurements"

# NOTE: the device variable, quoted by Grafana
DEVICE = "source in ($device)"

# NOTE: only registers both schneider-iEM3xxx and abb-B2x have
TOTAL_POWER = " + ".join(
  json_field(f"activePowerL{phase}NetT0_W") for phase in (1, 2, 3))
IMPORT = json_field("activeEnergyTotalImportT0_Wh")
IMPORT_T1 = json_field("activeEnergyTotalImportT1_Wh")
IMPORT_T2 = json_field("activeEnergyTotalImportT2_Wh")

PHASES = [
  ("L1", SeriesColor.S1),
  ("L2", SeriesColor.S2),
  ("L3", SeriesColor.S3),
]

def phases(prefix: str, suffix: str) -> str:
  return average_per_interval(
    TABLE,
    [(name, json_field(f"{prefix}{name}{suffix}")) for name, _ in PHASES],
    DEVICE,
  )

DASHBOARD = dashboard(
  "pidgeon-meters",
  "Meters",
  ("Per-meter electrical measurements from the measurements table."
   " Only uses registers shared by schneider-iEM3xxx and abb-B2x."),
  panels=[
    row(
      "$device",
      pos(0, 0, 24, 1),
      repeat="device",
    ),
    stat(
      "Active power now",
      latest(
        TABLE,
        TOTAL_POWER,
        "Active power",
        f"{DEVICE} and $__timeFilter(timestamp)",
      ),
      Unit.WATT,
      pos(0, 1, 6, 4),
    ),
    stat(
      "Imported today",
      f"""
select max({IMPORT}) - min({IMPORT}) as "Imported today"
from measurements
where {DEVICE}
  and timestamp >= time_bucket('1 day', now(), '{TIMEZONE}')
""",
      Unit.WATT_HOUR,
      pos(6, 1, 6, 4),
      description=f"Active energy import since midnight ({TIMEZONE}).",
    ),
    stat(
      "Import counter",
      latest(TABLE, IMPORT, "Import counter", DEVICE),
      Unit.WATT_HOUR,
      pos(12, 1, 6, 4),
      description="Meter's cumulative active energy import register.",
    ),
    stat(
      "Last measurement",
      latest(
        TABLE,
        "extract(epoch from now() - timestamp)",
        "Last measurement",
        DEVICE,
      ),
      Unit.SECONDS,
      pos(18, 1, 6, 4),
      description="Time since the newest stored measurement of this meter.",
      steps=thresholds(
        StatusColor.GOOD,
        (300, StatusColor.WARNING),
        (3600, StatusColor.CRITICAL),
      ),
      decimals=0,
    ),
    timeseries(
      "Voltage",
      phases("voltage", "AnyT0_V"),
      Unit.VOLT,
      pos(0, 5, 8, 8),
      series=PHASES,
    ),
    timeseries(
      "Current",
      phases("current", "AnyT0_A"),
      Unit.AMPERE,
      pos(8, 5, 8, 8),
      series=PHASES,
      min_=0,
    ),
    timeseries(
      "Active power",
      phases("activePower", "NetT0_W"),
      Unit.WATT,
      pos(16, 5, 8, 8),
      series=PHASES,
    ),
    timeseries(
      "Daily active energy import",
      f"""
select
  time_bucket('1 day', timestamp, '{TIMEZONE}') as time,
  max({IMPORT_T1})
    - min({IMPORT_T1}) as "Tariff 1",
  max({IMPORT_T2})
    - min({IMPORT_T2}) as "Tariff 2"
from measurements
where {DEVICE}
  and timestamp >= time_bucket('1 day', $__timeFrom()::timestamptz, '{TIMEZONE}')
  and timestamp < $__timeTo()
group by 1
order by 1
""",
      Unit.WATT_HOUR,
      pos(0, 13, 24, 8),
      series=[
        ("Tariff 1", SeriesColor.S1),
        ("Tariff 2", SeriesColor.S2),
      ],
      description=(
        f"Difference of the tariff counters per calendar day in {TIMEZONE}."),
      draw_style=DrawStyle.BARS,
      stacking=StackingMode.NORMAL,
      min_=0,
      interval="1d",
      time_from="7d",
    ),
  ],
  variables=[
    query_variable(
      "device",
      "Meter",
      ("select id as __value, id || ' (' || kind || ')' as __text"
       " from devices order by id"),
      multi=True,
      include_all=True,
    ),
  ],
)
