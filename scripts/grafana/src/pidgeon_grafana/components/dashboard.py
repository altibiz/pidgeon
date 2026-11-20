"""The dashboard around the panels: links, annotations and variables."""

from collections.abc import Sequence

from pidgeon_grafana.components.field_helpers import target
from pidgeon_grafana.components.grafana_enums import (
  DataSourceType,
  GraphTooltip,
  LinkIcon,
  LinkType,
  QueryFormat,
  RefreshInterval,
  VariableHide,
  VariableRefresh,
  VariableSort,
  VariableType,
  WeekStart,
)
from pidgeon_grafana.shared import DATA_SOURCE, TAG, Color, Json

# NOTE: Grafana adds it to every dashboard, so it is kept to avoid a diff
BUILT_IN_ANNOTATION: Json = {
  "builtIn": 1,
  "datasource": {
    "type": DataSourceType.GRAFANA,
    "uid": "-- Grafana --",
  },
  "enable": True,
  "hide": True,
  "iconColor": "rgba(0, 211, 255, 1)",
  "name": "Annotations & Alerts",
  "type": "dashboard",
}


def dashboard_link(
  title: str,
  tags: Sequence[str],
  link_type: LinkType = LinkType.DASHBOARDS,
  icon: LinkIcon = LinkIcon.EXTERNAL_LINK,
  url: str = "",
  tooltip_text: str = "",
  as_dropdown: bool = False,
  include_vars: bool = False,
  keep_time: bool = True,
  target_blank: bool = False,
) -> Json:
  return {
    "title": title,
    "type": link_type,
    "tags": list(tags),
    "asDropdown": as_dropdown,
    "includeVars": include_vars,
    "keepTime": keep_time,
    "targetBlank": target_blank,
    "icon": icon,
    "tooltip": tooltip_text,
    "url": url,
  }


def annotation(
  name: str,
  sql: str,
  icon_color: Color,
  enable: bool = True,
  hide: bool = False,
) -> Json:
  return {
    "name": name,
    "datasource": DATA_SOURCE,
    "enable": enable,
    "hide": hide,
    "iconColor": icon_color,
    "target": target(sql, QueryFormat.TABLE, ref_id="Anno"),
  }


def query_variable(
  name: str,
  label: str,
  sql: str,
  multi: bool = False,
  include_all: bool = False,
  refresh: VariableRefresh = VariableRefresh.ON_DASHBOARD_LOAD,
  sort: VariableSort = VariableSort.DISABLED,
  hide: VariableHide = VariableHide.NONE,
) -> Json:
  sql = sql.strip()
  current: Json = {}
  if include_all:
    current = {
      "selected": True,
      "text": ["All"] if multi else "All",
      "value": ["$__all"] if multi else "$__all",
    }
  return {
    "name": name,
    "label": label,
    "type": VariableType.QUERY,
    "datasource": DATA_SOURCE,
    "definition": sql,
    "query": sql,
    "refresh": refresh,
    "multi": multi,
    "includeAll": include_all,
    "current": current,
    "options": [],
    "regex": "",
    "sort": sort,
    "hide": hide,
    "skipUrlSync": False,
  }


def dashboard(
  uid: str,
  title: str,
  description: str,
  panels: Sequence[Json],
  variables: Sequence[Json] = (),
  annotations: Sequence[Json] = (),
  time_from: str = "now-24h",
  refresh: RefreshInterval = RefreshInterval.ONE_MINUTE,
  graph_tooltip: GraphTooltip = GraphTooltip.SHARED_CROSSHAIR,
  week_start: WeekStart = WeekStart.DEFAULT,
) -> Json:
  return {
    "uid": uid,
    # NOTE: Grafana assigns the id of provisioned dashboards
    "id": None,
    "title": title,
    "description": description,
    "tags": [TAG],
    "editable": True,
    "graphTooltip": graph_tooltip,
    "refresh": refresh,
    "schemaVersion": 39,
    "version": 1,
    # NOTE: empty is the viewer's browser time zone
    "timezone": "",
    "time": {
      "from": time_from,
      "to": "now",
    },
    "timepicker": {},
    "fiscalYearStartMonth": 0,
    "liveNow": False,
    "weekStart": week_start,
    "links": [
      dashboard_link("Pidgeon", [TAG]),
    ],
    "annotations": {
      "list": [
        BUILT_IN_ANNOTATION,
        *annotations,
      ],
    },
    "templating": {
      "list": list(variables),
    },
    "panels": list(panels),
  }
