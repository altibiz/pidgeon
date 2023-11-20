import asyncio
from typing import Any, List, Optional, Tuple
from pull import PullClient
from args import Args
from device import DeviceType


async def main():
  args = Args()

  client = PullClient(
    ip_address=args.ip_address(),
    slave_id=args.slave_id(),
  )

  if args.device_type() == DeviceType.abb:
    while True:
      print_measurement(DeviceType.abb, [
        (
          "Serial number",
          await client.read(
            register=0x8900,
            count=2,
            convert=PullClient.to_uint32,
          ),
        ),
        (
          "Mapping version",
          await client.read(
            register=0x8910,
            count=1,
            convert=PullClient.to_raw_bytes,
          ),
        ),
        (
          "Type designation",
          await client.read(
            register=0x8960,
            count=6,
            convert=PullClient.to_ascii,
          ),
        ),
        (
          "Power",
          await client.read(
            register=0x5B14,
            count=2,
            convert=PullClient.to_sint32,
          ),
        ),
      ])

  if args.device_type() == DeviceType.schneider:
    while True:
      print_measurement(DeviceType.schneider, [(
        "Model",
        await client.read(
          register=0x0031,
          count=20,
          convert=PullClient.to_utf8,
        ),
      ),
                                               (
                                                 "Serial number",
                                                 await client.read(
                                                   register=0x0081,
                                                   count=2,
                                                   convert=PullClient.to_uint32,
                                                 ),
                                               ),
                                               ("Power", await client.read(
                                                 register=0x0BF3,
                                                 count=2,
                                                 convert=PullClient.to_float32,
                                               ))])


def print_measurement(device_type: DeviceType,
                      registers: List[Tuple[str, Optional[Any]]]):
  print("Reading", device_type)
  for register in registers:
    print(register[0], register[1])
  print("\n")


if __name__ == "__main__":
  asyncio.run(main())
