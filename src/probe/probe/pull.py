import struct
import asyncio
from typing import Any, Callable, Coroutine, Optional, TypeVar, Union, List, cast
from pymodbus.client import AsyncModbusTcpClient
from pymodbus.framer import ModbusRtuFramer
from pymodbus.pdu import ModbusResponse

TRead = TypeVar("TRead")


class PullClient:

  def __init__(
    self,
    ip_address: str,
    slave_id: int,
  ):
    self.__ip_address = ip_address
    self.__slave_id = slave_id
    self.__modbus_connected = False
    self.__modbus_client = AsyncModbusTcpClient(
      host=self.__ip_address,
      port=502,
      framer=ModbusRtuFramer,
    )

  def __del__(self):
    self.__modbus_client.close()

  async def read(
    self,
    register: int,
    count: int,
    convert: Callable[..., TRead],
  ) -> Union[None, TRead]:
    await asyncio.sleep(1)

    if not self.__modbus_connected:
      try:
        await self.__modbus_client.connect()
      except Exception as exception:
        print(exception)
        return None

    try:
      response = await cast(
        Coroutine[Any, Any, ModbusResponse],
        self.__modbus_client.read_holding_registers(
          address=register,
          count=count,
          slave=self.__slave_id,
        ))
      if response.isError():
        print(response)
        return None

      value = convert(*cast(
        list[int], response.registers))  # pyright: ignore unknownMemberType
      return value
    except Exception as exception:
      print(exception)
      return None

  @staticmethod
  def multiplied_by(converter: Callable[..., Union[int, float]],
                    multiplier: float) -> Callable[..., float]:

    def result(*registers: int) -> float:
      converted = converter(*registers)
      multiplied = converted * multiplier
      return multiplied

    return result

  @staticmethod
  def to_uint32(first: int, second: int) -> int:
    return (first << 16) | second

  @staticmethod
  def to_uint64(first: int, second: int, third: int, fourth: int) -> int:
    return (first << 48) | (second << 32) | (third << 16) | fourth

  @staticmethod
  def to_sint32(upper_half: int, lower_half: int) -> int:
    combined_value = (upper_half << 16) | lower_half

    if combined_value & 0x80000000:
      combined_value -= 0x100000000

    return combined_value

  @staticmethod
  def to_sint16(register: int) -> int:
    return register if register < 0x8000 else register - 0x10000

  @staticmethod
  def to_float32(upper_half: int, lower_half: int) -> float:
    upper_bytes = struct.pack("!H", upper_half)
    lower_bytes = struct.pack("!H", lower_half)
    return struct.unpack("!f", upper_bytes + lower_bytes)[0]

  @staticmethod
  def to_raw_bytes(*registers: int) -> List[int]:
    return [
      uint8 for uint16 in registers
      for uint8 in [(uint16 >> 8) & 0xFF, uint16 & 0xFF]
    ]

  @staticmethod
  def to_bytes(*registers: int) -> bytes:
    return bytes(PullClient.to_raw_bytes(*registers))

  @staticmethod
  def to_ascii(*registers: int) -> str:
    return PullClient.to_bytes(*registers).decode("ascii")

  @staticmethod
  def to_utf8(*registers: int) -> str:
    return PullClient.to_bytes(*registers).decode("utf8")
