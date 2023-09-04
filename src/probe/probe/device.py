from __future__ import annotations
from enum import Enum
from typing import Optional


class DeviceType(Enum):
  abb = 'abb'

  def __str__(self):
    return self.value

  @staticmethod
  def from_string(string: str) -> Optional[DeviceType]:
    try:
      return DeviceType[string]
    except KeyError:
      return None
