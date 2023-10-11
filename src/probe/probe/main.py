import asyncio
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
    print(
      "Serial number",
      await client.read(
        register=0x8900,
        count=2,
        convert=PullClient.to_uint32,
      ),
    )
    print(
      "Mapping version",
      await client.read(
        register=0x8910,
        count=1,
        convert=PullClient.to_raw_bytes,
      ),
    )
    print(
      "Type designation",
      await client.read(
        register=0x8960,
        count=6,
        convert=PullClient.to_ascii,
      ),
    )

  if args.device_type() == DeviceType.schneider:
    print(
      "Model",
      await client.read(
        register=0x0031,
        count=20,
        convert=PullClient.to_utf8,
      ),
    )
    print(
      "Serial number",
      await client.read(
        register=0x0081,
        count=2,
        convert=PullClient.to_uint32,
      ),
    )


if __name__ == "__main__":
  asyncio.run(main())
