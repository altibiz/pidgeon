from collections.abc import Sequence
from pidgeon_grafana.components.grafana_enums import TimeGroupFill

def time_group(
  fill: TimeGroupFill | None = None,
  column: str = "timestamp",
) -> str:
  if fill is None:
    return f"$__timeGroupAlias({column}, $__interval)"
  return f"$__timeGroupAlias({column}, $__interval, {fill})"


def json_field(key: str, column: str = "data") -> str:
  return f"({column}->>'{key}')::float8"


def average_per_interval(
  table: str,
  columns: Sequence[tuple[str, str]],
  condition: str | None = None,
) -> str:
  select = ",\n  ".join(f'avg({expression}) as "{name}"'
                        for name, expression in columns)
  where = "$__timeFilter(timestamp)"
  if condition is not None:
    where += f" and {condition}"
  return f"""
          select
            {time_group(TimeGroupFill.NULL)},
            {select}
          from {table}
          where {where}
          group by 1
          order by 1
          """


def latest(
  table: str,
  expression: str,
  name: str,
  condition: str | None = None,
) -> str:
  lines = [
    f'select {expression} as "{name}"',
    f"from {table}",
    *([] if condition is None else [f"where {condition}"]),
    "order by timestamp desc",
    "limit 1",
  ]
  return "\n".join(lines)
