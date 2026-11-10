# NOTE: enums used in grafana field settings

from enum import IntEnum, StrEnum

# -----------------------------------------------------------------------------
# data sources and queries
# -----------------------------------------------------------------------------


class DataSourceType(StrEnum):
  POSTGRES = "grafana-postgresql-datasource"
  # NOTE: built-in, used by the "Annotations & Alerts" annotation
  GRAFANA = "grafana"


class QueryFormat(StrEnum):
  TIME_SERIES = "time_series"
  TABLE = "table"


class EditorMode(StrEnum):
  CODE = "code"
  BUILDER = "builder"


class TimeGroupFill(StrEnum):
  NULL = "NULL"
  PREVIOUS = "previous"
  ZERO = "0"


# -----------------------------------------------------------------------------
# panels
# -----------------------------------------------------------------------------


class PanelType(StrEnum):
  ROW = "row"
  TIMESERIES = "timeseries"
  STAT = "stat"
  GAUGE = "gauge"
  BAR_GAUGE = "bargauge"
  BAR_CHART = "barchart"
  TABLE = "table"
  STATE_TIMELINE = "state-timeline"
  STATUS_HISTORY = "status-history"
  PIE_CHART = "piechart"
  HISTOGRAM = "histogram"
  HEATMAP = "heatmap"
  TREND = "trend"
  XY_CHART = "xychart"
  CANDLESTICK = "candlestick"
  TEXT = "text"
  LOGS = "logs"
  NEWS = "news"
  DASHBOARD_LIST = "dashlist"
  ALERT_LIST = "alertlist"
  ANNOTATION_LIST = "annolist"
  GEOMAP = "geomap"
  CANVAS = "canvas"
  NODE_GRAPH = "nodeGraph"
  TRACES = "traces"
  FLAME_GRAPH = "flamegraph"


# -----------------------------------------------------------------------------
# field config
# -----------------------------------------------------------------------------


class Unit(StrEnum):
  # generic
  NONE = "none"
  SHORT = "short"
  PERCENT = "percent"
  PERCENT_UNIT = "percentunit"

  # electrical
  WATT = "watt"
  KILOWATT = "kwatt"
  MEGAWATT = "megwatt"
  VOLT_AMPERE = "voltamp"
  KILOVOLT_AMPERE = "kvoltamp"
  VOLT_AMPERE_REACTIVE = "voltampreact"
  KILOVOLT_AMPERE_REACTIVE = "kvoltampreact"
  WATT_HOUR = "watth"
  KILOWATT_HOUR = "kwatth"
  JOULE = "joule"
  AMPERE = "amp"
  MILLIAMPERE = "mamp"
  VOLT = "volt"
  MILLIVOLT = "mvolt"
  KILOVOLT = "kvolt"
  HERTZ = "hertz"

  # temperature
  CELSIUS = "celsius"
  FAHRENHEIT = "fahrenheit"
  KELVIN = "kelvin"

  # data
  BYTES = "bytes"
  DECIMAL_BYTES = "decbytes"
  BITS = "bits"

  # time
  MILLISECONDS = "ms"
  SECONDS = "s"
  MINUTES = "m"
  HOURS = "h"
  DAYS = "d"
  # NOTE: seconds shown as "1 day, 2 hours"
  DURATION_SECONDS = "dtdurations"
  DURATION_MILLISECONDS = "dtdurationms"
  DATE_TIME_ISO = "dateTimeAsIso"
  DATE_TIME_FROM_NOW = "dateTimeFromNow"


class FieldColorMode(StrEnum):
  FIXED = "fixed"
  SHADES = "shades"
  THRESHOLDS = "thresholds"
  PALETTE_CLASSIC = "palette-classic"
  PALETTE_CLASSIC_BY_NAME = "palette-classic-by-name"
  CONTINUOUS_GREEN_YELLOW_RED = "continuous-GrYlRd"
  CONTINUOUS_RED_YELLOW_GREEN = "continuous-RdYlGr"
  CONTINUOUS_BLUE_YELLOW_RED = "continuous-BlYlRd"
  CONTINUOUS_YELLOW_RED = "continuous-YlRd"
  CONTINUOUS_BLUE_PURPLE = "continuous-BlPu"
  CONTINUOUS_YELLOW_BLUE = "continuous-YlBl"
  CONTINUOUS_BLUES = "continuous-blues"
  CONTINUOUS_REDS = "continuous-reds"
  CONTINUOUS_GREENS = "continuous-greens"
  CONTINUOUS_PURPLES = "continuous-purples"


class ThresholdsMode(StrEnum):
  ABSOLUTE = "absolute"
  PERCENTAGE = "percentage"


class MappingType(StrEnum):
  VALUE = "value"
  RANGE = "range"
  REGEX = "regex"
  SPECIAL = "special"


class SpecialValueMatch(StrEnum):
  TRUE = "true"
  FALSE = "false"
  NULL = "null"
  NAN = "nan"
  NULL_AND_NAN = "null+nan"
  EMPTY = "empty"


class Matcher(StrEnum):
  BY_NAME = "byName"
  BY_REGEXP = "byRegexp"
  BY_TYPE = "byType"
  BY_FRAME_REF_ID = "byFrameRefID"


class FieldType(StrEnum):
  TIME = "time"
  NUMBER = "number"
  STRING = "string"
  BOOLEAN = "boolean"
  OTHER = "other"


class FieldProperty(StrEnum):
  COLOR = "color"
  UNIT = "unit"
  DECIMALS = "decimals"
  MIN = "min"
  MAX = "max"
  DISPLAY_NAME = "displayName"
  NO_VALUE = "noValue"
  MAPPINGS = "mappings"
  THRESHOLDS = "thresholds"
  CUSTOM_WIDTH = "custom.width"
  CUSTOM_ALIGN = "custom.align"
  CUSTOM_CELL_OPTIONS = "custom.cellOptions"
  CUSTOM_HIDDEN = "custom.hidden"
  CUSTOM_DRAW_STYLE = "custom.drawStyle"
  CUSTOM_LINE_WIDTH = "custom.lineWidth"
  CUSTOM_FILL_OPACITY = "custom.fillOpacity"
  CUSTOM_STACKING = "custom.stacking"
  CUSTOM_AXIS_PLACEMENT = "custom.axisPlacement"
  CUSTOM_HIDE_FROM = "custom.hideFrom"


class Reducer(StrEnum):
  LAST_NOT_NULL = "lastNotNull"
  LAST = "last"
  FIRST_NOT_NULL = "firstNotNull"
  FIRST = "first"
  MIN = "min"
  MAX = "max"
  MEAN = "mean"
  MEDIAN = "median"
  SUM = "sum"
  COUNT = "count"
  RANGE = "range"
  DELTA = "delta"
  DIFFERENCE = "diff"
  DISTINCT_COUNT = "distinctCount"
  CHANGE_COUNT = "changeCount"
  STANDARD_DEVIATION = "stdDev"
  VARIANCE = "variance"
  ALL_VALUES = "allValues"
  UNIQUE_VALUES = "uniqueValues"


# -----------------------------------------------------------------------------
# time series
# -----------------------------------------------------------------------------


class DrawStyle(StrEnum):
  LINE = "line"
  BARS = "bars"
  POINTS = "points"


class LineInterpolation(StrEnum):
  LINEAR = "linear"
  SMOOTH = "smooth"
  STEP_BEFORE = "stepBefore"
  STEP_AFTER = "stepAfter"


class Visibility(StrEnum):
  AUTO = "auto"
  NEVER = "never"
  ALWAYS = "always"


class GradientMode(StrEnum):
  NONE = "none"
  OPACITY = "opacity"
  HUE = "hue"
  SCHEME = "scheme"


class StackingMode(StrEnum):
  NONE = "none"
  NORMAL = "normal"
  PERCENT = "percent"


class AxisPlacement(StrEnum):
  AUTO = "auto"
  TOP = "top"
  RIGHT = "right"
  BOTTOM = "bottom"
  LEFT = "left"
  HIDDEN = "hidden"


class ScaleDistribution(StrEnum):
  LINEAR = "linear"
  LOG = "log"
  ORDINAL = "ordinal"
  SYMLOG = "symlog"


class ThresholdsStyle(StrEnum):
  OFF = "off"
  LINE = "line"
  DASHED = "dashed"
  AREA = "area"
  LINE_AND_AREA = "line+area"
  DASHED_AND_AREA = "dashed+area"
  SERIES = "series"


class BarAlignment(IntEnum):
  BEFORE = -1
  CENTER = 0
  AFTER = 1


# -----------------------------------------------------------------------------
# legend and tooltip
# -----------------------------------------------------------------------------


class LegendDisplayMode(StrEnum):
  LIST = "list"
  TABLE = "table"
  HIDDEN = "hidden"


class LegendPlacement(StrEnum):
  BOTTOM = "bottom"
  RIGHT = "right"


class TooltipMode(StrEnum):
  SINGLE = "single"
  MULTI = "multi"
  NONE = "none"


class SortOrder(StrEnum):
  ASCENDING = "asc"
  DESCENDING = "desc"
  NONE = "none"


# -----------------------------------------------------------------------------
# stat
# -----------------------------------------------------------------------------


class StatColorMode(StrEnum):
  VALUE = "value"
  BACKGROUND = "background"
  BACKGROUND_SOLID = "background_solid"
  NONE = "none"


class StatGraphMode(StrEnum):
  NONE = "none"
  LINE = "line"
  AREA = "area"


class StatJustifyMode(StrEnum):
  AUTO = "auto"
  CENTER = "center"


class StatTextMode(StrEnum):
  AUTO = "auto"
  VALUE = "value"
  VALUE_AND_NAME = "value_and_name"
  NAME = "name"
  NONE = "none"


class Orientation(StrEnum):
  AUTO = "auto"
  VERTICAL = "vertical"
  HORIZONTAL = "horizontal"


# -----------------------------------------------------------------------------
# state timeline
# -----------------------------------------------------------------------------


class TimelineValueAlignment(StrEnum):
  LEFT = "left"
  CENTER = "center"
  RIGHT = "right"


# -----------------------------------------------------------------------------
# table
# -----------------------------------------------------------------------------


class CellDisplayMode(StrEnum):
  AUTO = "auto"
  COLOR_TEXT = "color-text"
  COLOR_BACKGROUND = "color-background"
  GAUGE = "gauge"
  SPARKLINE = "sparkline"
  JSON_VIEW = "json-view"
  IMAGE = "image"
  DATA_LINKS = "data-links"


class CellBackgroundMode(StrEnum):
  BASIC = "basic"
  GRADIENT = "gradient"


class CellHeight(StrEnum):
  SMALL = "sm"
  MEDIUM = "md"
  LARGE = "lg"


class TextAlignment(StrEnum):
  AUTO = "auto"
  LEFT = "left"
  CENTER = "center"
  RIGHT = "right"


# -----------------------------------------------------------------------------
# dashboard
# -----------------------------------------------------------------------------


class GraphTooltip(IntEnum):
  DEFAULT = 0
  SHARED_CROSSHAIR = 1
  SHARED_TOOLTIP = 2


class RefreshInterval(StrEnum):
  FIVE_SECONDS = "5s"
  TEN_SECONDS = "10s"
  THIRTY_SECONDS = "30s"
  ONE_MINUTE = "1m"
  FIVE_MINUTES = "5m"
  FIFTEEN_MINUTES = "15m"
  THIRTY_MINUTES = "30m"
  ONE_HOUR = "1h"
  TWO_HOURS = "2h"
  ONE_DAY = "1d"


class WeekStart(StrEnum):
  DEFAULT = ""
  SATURDAY = "saturday"
  SUNDAY = "sunday"
  MONDAY = "monday"


class LinkType(StrEnum):
  LINK = "link"
  DASHBOARDS = "dashboards"


class LinkIcon(StrEnum):
  EXTERNAL_LINK = "external link"
  DASHBOARD = "dashboard"
  QUESTION = "question"
  INFO = "info"
  BOLT = "bolt"
  DOC = "doc"
  CLOUD = "cloud"


class VariableType(StrEnum):
  QUERY = "query"
  CUSTOM = "custom"
  CONSTANT = "constant"
  TEXT_BOX = "textbox"
  INTERVAL = "interval"
  DATA_SOURCE = "datasource"
  AD_HOC = "adhoc"


class VariableRefresh(IntEnum):
  NEVER = 0
  ON_DASHBOARD_LOAD = 1
  ON_TIME_RANGE_CHANGE = 2


class VariableHide(IntEnum):
  NONE = 0
  LABEL = 1
  VARIABLE = 2


class VariableSort(IntEnum):
  DISABLED = 0
  ALPHABETICAL_ASCENDING = 1
  ALPHABETICAL_DESCENDING = 2
  NUMERICAL_ASCENDING = 3
  NUMERICAL_DESCENDING = 4
  CASE_INSENSITIVE_ASCENDING = 5
  CASE_INSENSITIVE_DESCENDING = 6
  NATURAL_ASCENDING = 7
  NATURAL_DESCENDING = 8
