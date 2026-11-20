from collections.abc import Sequence

from pidgeon_grafana.components.field_helpers import (
  cell_options,
  color,
  field_config,
  fixed,
  optional,
  target,
  thresholds,
)
from pidgeon_grafana.components.grafana_enums import (
  AxisPlacement,
  BarAlignment,
  CellDisplayMode,
  CellHeight,
  DrawStyle,
  FieldColorMode,
  GradientMode,
  LegendDisplayMode,
  LegendPlacement,
  LineInterpolation,
  Orientation,
  PanelType,
  QueryFormat,
  Reducer,
  ScaleDistribution,
  SortOrder,
  StackingMode,
  StatColorMode,
  StatGraphMode,
  StatJustifyMode,
  StatTextMode,
  TextAlignment,
  ThresholdsStyle,
  TimelineValueAlignment,
  TooltipMode,
  Unit,
  Visibility,
)
from pidgeon_grafana.shared import (
  DATA_SOURCE,
  Color,
  Json,
  SeriesColor,
  StatusColor,
)


def legend(
  show: bool = True,
  calcs: Sequence[Reducer] = (),
  mode: LegendDisplayMode = LegendDisplayMode.LIST,
  placement: LegendPlacement = LegendPlacement.BOTTOM,
) -> Json:
  return {
    "showLegend": show,
    "displayMode": mode,
    "placement": placement,
    "calcs": list(calcs),
  }


def tooltip(
  mode: TooltipMode = TooltipMode.MULTI,
  sort: SortOrder = SortOrder.NONE,
) -> Json:
  return {
    "mode": mode,
    "sort": sort,
  }


def panel(
  panel_type: PanelType,
  title: str,
  grid: Json,
  targets: Sequence[Json],
  defaults: Json,
  options: Json,
  overrides: Sequence[Json] = (),
  description: str = "",
  interval: str | None = None,
) -> Json:
  return {
    "type": panel_type,
    "title": title,
    "description": description,
    "datasource": DATA_SOURCE,
    "gridPos": grid,
    "fieldConfig": field_config(defaults, overrides),
    "options": options,
    "targets": list(targets),
    **optional(interval=interval),
  }


def row(
  title: str,
  grid: Json,
  repeat: str | None = None,
  collapsed: bool = False,
) -> Json:
  return {
    "type": PanelType.ROW,
    "title": title,
    "collapsed": collapsed,
    "gridPos": grid,
    "panels": [],
    **optional(repeat=repeat),
  }


def stat(
  title: str,
  sql: str,
  unit: Unit,
  grid: Json,
  description: str = "",
  steps: Json | None = None,
  mappings: Sequence[Json] = (),
  fmt: QueryFormat = QueryFormat.TABLE,
  reducer: Reducer = Reducer.LAST_NOT_NULL,
  color_mode: StatColorMode | None = None,
  graph_mode: StatGraphMode = StatGraphMode.NONE,
  text_mode: StatTextMode = StatTextMode.AUTO,
  justify_mode: StatJustifyMode = StatJustifyMode.AUTO,
  orientation: Orientation = Orientation.AUTO,
  decimals: int | None = None,
  interval: str = "1m",
) -> Json:
  colored = steps is not None or len(mappings) > 0
  default_color_mode = (StatColorMode.BACKGROUND
                        if colored else StatColorMode.NONE)
  return panel(
    PanelType.STAT,
    title,
    grid,
    targets=[target(sql, fmt)],
    defaults={
      "unit": unit,
      "color": color(FieldColorMode.THRESHOLDS),
      "thresholds": steps or thresholds(StatusColor.NEUTRAL),
      "mappings": list(mappings),
      **optional(decimals=decimals),
    },
    options={
      "reduceOptions": {
        "calcs": [reducer],
        "fields": "",
        "values": False,
      },
      "colorMode": color_mode or default_color_mode,
      "graphMode": graph_mode,
      "justifyMode": justify_mode,
      "orientation": orientation,
      "textMode": text_mode,
      "wideLayout": True,
      "showPercentChange": False,
    },
    description=description,
    interval=interval,
  )


def timeseries(
  title: str,
  sql: str,
  unit: Unit,
  grid: Json,
  series: Sequence[tuple[str, Color]] = (),
  description: str = "",
  draw_style: DrawStyle = DrawStyle.LINE,
  stacking: StackingMode = StackingMode.NONE,
  interpolation: LineInterpolation = LineInterpolation.LINEAR,
  show_points: Visibility = Visibility.NEVER,
  gradient: GradientMode = GradientMode.NONE,
  axis: AxisPlacement = AxisPlacement.AUTO,
  scale: ScaleDistribution = ScaleDistribution.LINEAR,
  thresholds_style: ThresholdsStyle = ThresholdsStyle.OFF,
  bar_alignment: BarAlignment = BarAlignment.CENTER,
  legend_options: Json | None = None,
  tooltip_options: Json | None = None,
  min_: float | None = None,
  max_: float | None = None,
  decimals: int | None = None,
  interval: str = "1m",
  time_from: str | None = None,
) -> Json:
  return panel(
    PanelType.TIMESERIES,
    title,
    grid,
    targets=[target(sql)],
    defaults={
      "unit": unit,
      "color": color(FieldColorMode.FIXED, SeriesColor.S1),
      "custom": {
        "drawStyle": draw_style,
        "lineWidth": 2,
        "fillOpacity": 90 if draw_style == DrawStyle.BARS else 0,
        "gradientMode": gradient,
        "showPoints": show_points,
        "pointSize": 8,
        "lineInterpolation": interpolation,
        "spanNulls": False,
        "barAlignment": bar_alignment,
        "axisPlacement": axis,
        "axisBorderShow": False,
        "axisCenteredZero": False,
        "scaleDistribution": {
          "type": scale,
        },
        "stacking": {
          "mode": stacking,
          "group": "A",
        },
        "thresholdsStyle": {
          "mode": thresholds_style,
        },
        "hideFrom": {
          "legend": False,
          "tooltip": False,
          "viz": False,
        },
      },
      **optional(min=min_, max=max_, decimals=decimals),
    },
    overrides=[fixed(name, series_color) for name, series_color in series],
    options={
      "legend": legend_options or legend(show=len(series) > 1),
      "tooltip": tooltip_options or tooltip(),
    },
    description=description,
    interval=interval,
  ) | optional(timeFrom=time_from)


def state_timeline(
  title: str,
  sql: str,
  grid: Json,
  mappings: Sequence[Json],
  description: str = "",
  show_value: Visibility = Visibility.NEVER,
  align_value: TimelineValueAlignment = TimelineValueAlignment.LEFT,
  merge_values: bool = True,
  row_height: float = 0.8,
  legend_options: Json | None = None,
  tooltip_options: Json | None = None,
  interval: str = "1m",
) -> Json:
  return panel(
    PanelType.STATE_TIMELINE,
    title,
    grid,
    targets=[target(sql)],
    defaults={
      # NOTE: thresholds color mode makes merged states ignore value mappings
      "color": color(FieldColorMode.FIXED, StatusColor.NEUTRAL),
      "mappings": list(mappings),
      "custom": {
        "fillOpacity": 80,
        "lineWidth": 0,
        "spanNulls": False,
      },
    },
    options={
      "showValue": show_value,
      "mergeValues": merge_values,
      "alignValue": align_value,
      "rowHeight": row_height,
      "legend": legend_options or legend(),
      "tooltip": tooltip_options or tooltip(TooltipMode.SINGLE),
    },
    description=description,
    interval=interval,
  )


def table(
  title: str,
  sql: str,
  grid: Json,
  overrides: Sequence[Json] = (),
  description: str = "",
  align: TextAlignment = TextAlignment.AUTO,
  display: CellDisplayMode = CellDisplayMode.AUTO,
  cell_height: CellHeight = CellHeight.SMALL,
  inspect: bool = False,
  sort_by: str | None = None,
  descending: bool = False,
) -> Json:
  sort = None if sort_by is None else [
    {
      "displayName": sort_by,
      "desc": descending,
    },
  ]
  return panel(
    PanelType.TABLE,
    title,
    grid,
    targets=[target(sql, QueryFormat.TABLE)],
    defaults={
      "custom": {
        "align": align,
        "cellOptions": cell_options(display),
        "inspect": inspect,
      },
      "mappings": [],
      "thresholds": thresholds(StatusColor.NEUTRAL),
    },
    overrides=overrides,
    options={
      "showHeader": True,
      "cellHeight": cell_height,
      "footer": {
        "show": False,
        "reducer": [Reducer.SUM],
        "fields": "",
        "countRows": False,
      },
      **optional(sortBy=sort),
    },
    description=description,
  )
