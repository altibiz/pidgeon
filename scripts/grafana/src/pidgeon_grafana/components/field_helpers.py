from collections.abc import Sequence
from typing import Any

from pidgeon_grafana.components.grafana_enums import (
  CellBackgroundMode,
  CellDisplayMode,
  EditorMode,
  FieldColorMode,
  FieldProperty,
  MappingType,
  Matcher,
  QueryFormat,
  SpecialValueMatch,
  ThresholdsMode,
)
from pidgeon_grafana.shared import DATA_SOURCE, Color, Json


def optional(**values: Any) -> Json:
  return {key: value for key, value in values.items() if value is not None}


def target(
  sql: str,
  fmt: QueryFormat = QueryFormat.TIME_SERIES,
  ref_id: str = "A",
  editor_mode: EditorMode = EditorMode.CODE,
) -> Json:
  return {
    "refId": ref_id,
    "datasource": DATA_SOURCE,
    "editorMode": editor_mode,
    "format": fmt,
    "rawQuery": True,
    "rawSql": sql.strip(),
  }


def pos(x: int, y: int, w: int, h: int) -> Json:
  # NOTE: dashboard is 24 cols wide
  return {
    "x": x,
    "y": y,
    "w": w,
    "h": h,
  }


def color(mode: FieldColorMode, fixed_color: Color | None = None) -> Json:
  return {
    "mode": mode,
    **optional(fixedColor=fixed_color),
  }


def threshold_step(step_color: Color, value: float | None) -> Json:
  return {
    "color": step_color,
    "value": value,
  }


def thresholds(
  base: Color,
  *steps: tuple[float, Color],
  mode: ThresholdsMode = ThresholdsMode.ABSOLUTE,
) -> Json:
  above = [threshold_step(step_color, value) for value, step_color in steps]
  return {
    "mode": mode,
    "steps": [
      threshold_step(base, None),
      *above,
    ],
  }


def value_mapping(*values: tuple[int | str, str, Color]) -> Json:
  return {
    "type": MappingType.VALUE,
    "options": {
      str(value): {
        "text": text,
        "color": mapped_color,
        "index": index,
      }
      for index, (value, text, mapped_color) in enumerate(values)
    },
  }


def range_mapping(
  start: float | None,
  end: float | None,
  text: str,
  mapped_color: Color,
  index: int = 0,
) -> Json:
  return {
    "type": MappingType.RANGE,
    "options": {
      "from": start,
      "to": end,
      "result": {
        "text": text,
        "color": mapped_color,
        "index": index,
      },
    },
  }


def special_mapping(
  match: SpecialValueMatch,
  text: str,
  mapped_color: Color,
  index: int = 0,
) -> Json:
  return {
    "type": MappingType.SPECIAL,
    "options": {
      "match": match,
      "result": {
        "text": text,
        "color": mapped_color,
        "index": index,
      },
    },
  }


def field_property(property_id: FieldProperty, value: Any) -> Json:
  return {
    "id": property_id,
    "value": value,
  }


def override(matcher: Matcher, options: str, *properties: Json) -> Json:
  return {
    "matcher": {
      "id": matcher,
      "options": options,
    },
    "properties": list(properties),
  }


def fixed(name: str, fixed_color: Color) -> Json:
  return override(
    Matcher.BY_NAME,
    name,
    field_property(
      FieldProperty.COLOR,
      color(FieldColorMode.FIXED, fixed_color),
    ),
  )


def cell_options(
  display: CellDisplayMode,
  background: CellBackgroundMode | None = None,
) -> Json:
  return {
    "type": display,
    **optional(mode=background),
  }


def field_config(defaults: Json, overrides: Sequence[Json] = ()) -> Json:
  return {
    "defaults": defaults,
    "overrides": list(overrides),
  }
