import asyncio
import pymodbus
from typing import Callable, TypeVar, Union, List


def to_uint32(first: int, second: int) -> int:
  return (first << 16) | second


def to_uint64(first: int, second: int, third: int, fourth: int) -> int:
  return (first << 48) | (second << 32) | (third << 16) | fourth


def to_float32(upper_half: int, lower_half: int) -> int:
  upper_bytes = struct.pack("!H", upper_half)
  lower_bytes = struct.pack("!H", lower_half)
  return struct.unpack("!f", upper_bytes + lower_bytes)[0]


def to_sint32(upper_half: int, lower_half: int) -> int:
  combined_value = (upper_half << 16) | lower_half

  if combined_value & 0x80000000:
    combined_value -= 0x100000000

  return combined_value


def to_sint16(register: int) -> int:
  return register if register < 0x8000 else register - 0x10000


def to_raw_bytes(*uint16s: int) -> List[int]:
  return [
    uint8 for uint16 in uint16s
    for uint8 in [(uint16 >> 8) & 0xFF, uint16 & 0xFF]
  ]


def to_bytes(*uint16s: int) -> bytes:
  return bytes(to_raw_bytes(*uint16s))


def to_ascii(*uint16s: int) -> str:
  return to_bytes(uint8s).decode('ascii')


def to_utf8(*uint16s: int) -> str:
  return to_bytes(uint8s).decode('utf8')


TRead = TypeVar('TRead')


class PullClient:

  def __init__(self, address: str):
    self.__address = address
    self.__client = pymodbus.client.AsyncModbusTcpClient("192.168.1.105")

  async def connect(self):
    await self.__client.connect()

  async def disconnect(self):
    self.__client.close()

  async def read(self, slave: int, register: int, count: int,
                 convert: Callable[*int, TRead]) -> Union[None, TRead]:
    try:
      response = await self.__client.read_holding_registers(
        register, size, slave)
      if response.isError():
        return None

      value = convert(*response.registers)
      return value
    except Exception as exception:
      return None


async def main():
  client = PullClient("192.168.1.105")
  await client.connect()

  print(await client.read(1, 0x8908, 8, to_ascii))

  await client.disconnect()


if __name__ == "__main__":
  asyncio.run(main())
