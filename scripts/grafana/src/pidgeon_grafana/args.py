from argparse import ArgumentParser
from pathlib import Path


class Args:

  def __init__(self):
    self.__parser = ArgumentParser(
      prog="pidgeon-grafana",
      description="Generate the pidgeon Grafana dashboards",
    )
    self.__parser.add_argument(
      "-o",
      "--out",
      required=True,
      type=Path,
      help="Directory the dashboard JSON files are written to",
    )
    self.__parser.add_argument(
      "-c",
      "--check",
      action="store_true",
      help="Write nothing and fail if a dashboard file is out of date",
    )
    self.__args = self.__parser.parse_args()

  def out(self) -> Path:
    return self.__args.out

  def check(self) -> bool:
    return self.__args.check
