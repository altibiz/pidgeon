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
        register=0x8908,
        count=8,
        convert=PullClient.to_ascii,
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


if __name__ == "__main__":
  asyncio.run(main())
