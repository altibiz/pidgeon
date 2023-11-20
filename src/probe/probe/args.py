import re
from typing import Callable, Optional
from argparse import ArgumentParser
from device import DeviceType
from func import compose


class Args:

  def __init__(self):
    self.__parser = ArgumentParser(
      prog="pidgeon-probe",
      description="Pidgeon probe",
    )

    self.__parser.add_argument(
      "-i",
      "--ip-address",
      required=True,
      type=Args.__regex(r"^((25[0-5]|(2[0-4]|1[0-9]|[1-9]|)[0-9])\.?\b){4}$"),
      help="IP address of the modbus gateway",
    )
    self.__parser.add_argument(
      "-s",
      "--slave-id",
      required=False,
      default=0,
      type=compose(Args.__regex("^25[0-5]|(2[0-4]|1[0-9]|[1-9]|)[0-9]$"), int),
      help="Slave ID of the modbus device",
    )
    self.__parser.add_argument(
      "-d",
      "--device-type",
      required=True,
      type=DeviceType.from_string,
      help="Type of the modbus device",
      choices=list(DeviceType),
    )

    self.__args = self.__parser.parse_args()

  def ip_address(self) -> str:
    return self.__args.ip_address

  def slave_id(self) -> int:
    return self.__args.slave_id

  def device_type(self) -> DeviceType:
    return self.__args.device_type

  @staticmethod
  def __regex(regex: str) -> Callable[[str], str]:
    compiled = re.compile(regex)

    def coerce(string: str) -> str:
      string = str(string)

      if not compiled.match(string):
        raise ValueError(f"string '{string}' did not match regex '{regex}'")

      return string

    return coerce
