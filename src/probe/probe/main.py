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
          name="Active power",
          register=0x5B14,
          count=2,
          convert=PullClient.to_sint32,
        ),
        Request(
          name="Active power export L1",
          register=0x546C,
          count=4,
          convert=PullClient.to_raw_bytes,
        ),
        Request(
          name="Reactive Power",
          register=0x5B1C,
          count=2,
          convert=PullClient.to_raw_bytes,
        ),
        Request(
          name="Reactive Import",
          register=0x500C,
          count=2,
          convert=PullClient.to_uint32,
        ),
        Request(
          name="Reactive Export",
          register=0x5010,
          count=2,
          convert=PullClient.to_uint32,
        ),
        Request(
          name="Reactive Net",
          register=0x5014,
          count=2,
          convert=PullClient.to_sint32,
        ),
        Request(
          name="Active Import",
          register=0x5000,
          count=2,
          convert=PullClient.to_uint32,
        ),
        Request(
          name="Active Export",
          register=0x5004,
          count=2,
          convert=PullClient.to_uint32,
        ),
        Request(
          name="Active Net",
          register=0x5008,
          count=2,
          convert=PullClient.to_sint32,
        ),
        Request(
          name="Tariff configuration",
          register=0x8C90,
          count=1,
          convert=PullClient.to_raw_bytes,
        ),
        Request(
          name="Tariff",
          register=0x8A07,
          count=1,
          convert=PullClient.to_raw_bytes,
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
          name="Active Power",
          register=0x0BF3,
          count=2,
          convert=PullClient.to_float32,
        ),
        Request(
          name="Active energy import total",
          register=0x0C83,
          count=4,
          convert=PullClient.to_sint64,
        ),
        Request(
          name="Active energy import L1",
          register=0x0DBD,
          count=4,
          convert=PullClient.to_sint64,
        ),
        Request(
          name="Active energy import L2",
          register=0x0DC1,
          count=4,
          convert=PullClient.to_sint64,
        ),
        Request(
          name="Active energy import L3",
          register=0x0DC5,
          count=4,
          convert=PullClient.to_sint64,
        ),
        Request(
          name="Tariff",
          register=0x105E,
          count=1,
          convert=PullClient.to_raw_bytes,
        ),
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
