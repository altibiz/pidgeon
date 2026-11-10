from enum import StrEnum
from typing import Any, TypeAlias
from pidgeon_grafana.components.grafana_enums import DataSourceType

Json: TypeAlias = dict[str, Any]

# NOTE: must match the uid in scripts/flake/pidgeonHost.nix and
# assets/grafana/dev/datasource.yml
DATA_SOURCE: Json = {
  "type": DataSourceType.POSTGRES,
  "uid": "pidgeon-db",
}

# NOTE: links every dashboard tagged with it to the others
TAG = "pidgeon"

# NOTE: mirrors schedule.timezone in assets/pidgeon/config.toml
TIMEZONE = "Europe/Zagreb"

# NOTE: series colors, to be used only for differentiating L1 - L3 series
class SeriesColor(StrEnum):
  S1 = "#3987e5"
  S2 = "#d95926"
  S3 = "#199e70"

# NOTE: state colors,
class StatusColor(StrEnum):
  GOOD = "#0ca30c"
  WARNING = "#fab219"
  CRITICAL = "#d03b3b"
  NEUTRAL = "#8e8e8e"
  TRANSPARENT = "transparent"


Color: TypeAlias = SeriesColor | StatusColor
