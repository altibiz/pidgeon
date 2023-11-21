import asyncio
import time
from typing import Any, List, NamedTuple, Callable
from pull import PullClient
from args import Args
from device import DeviceType


class Request(NamedTuple):
  name: str
  register: int
  count: int
  convert: Callable[..., Any]


async def main():
  args = Args()

  client = PullClient(
    ip_address=args.ip_address(),
    slave_id=args.slave_id(),
  )

  if args.device_type() == DeviceType.abb:
    while True:
      await measure(client, DeviceType.abb, [
        Request(
          name="Type designation",
          register=0x8960,
          count=6,
          convert=PullClient.to_ascii,
        ),
        Request(
          name="Serial number",
          register=0x8900,
          count=2,
          convert=PullClient.to_uint32,
        ),
        Request(
          name="Power",
          register=0x5B14,
          count=2,
          convert=PullClient.to_sint32,
        ),
      ])

  if args.device_type() == DeviceType.schneider:
    while True:
      await measure(client, DeviceType.schneider, [
        Request(
          name="Model",
          register=0x0031,
          count=20,
          convert=PullClient.to_utf8,
        ),
        Request(
          name="Serial number",
          register=0x0081,
          count=2,
          convert=PullClient.to_uint32,
        ),
        Request(
          name="Power",
          register=0x0BF3,
          count=2,
          convert=PullClient.to_float32,
        )
      ])


async def measure(client: PullClient, device_type: DeviceType,
                  requests: List[Request]):
  print("Reading", device_type)
  start = time.time()
  for request in requests:
    value = await client.read(
      register=request.register,
      count=request.count,
      convert=request.convert,
    )
    print(request.name, value)
  end = time.time()
  print("took", end - start, "\n")


if __name__ == "__main__":
  asyncio.run(main())
