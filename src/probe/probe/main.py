import asyncio
import time
from typing import Any, List, NamedTuple, Callable, Union
from client import Client
from args import Args
from device import DeviceType


class ReadRequest(NamedTuple):
  name: str
  register: int
  size: int
  convert: Callable[..., Any]


class WriteRequest(NamedTuple):
  name: str
  register: int
  values: list[int]


async def main():
  args = Args()

  client = Client(
    ip_address=args.ip_address(),
    slave_id=args.slave_id(),
  )

  if args.device_type() == DeviceType.abb:
    while True:
      await execute(
        client,
        DeviceType.abb,
        [
          ReadRequest(
            name="Type designation",
            register=0x8960,
            size=6,
            convert=Client.to_ascii,
          ),
          ReadRequest(
            name="Serial number",
            register=0x8900,
            size=2,
            convert=Client.to_uint32,
          ),
          ReadRequest(
            name="Active power",
            register=0x5B14,
            size=2,
            convert=Client.to_sint32,
          ),
          ReadRequest(
            name="Active power export L1",
            register=0x546C,
            size=4,
            convert=Client.to_hex,
          ),
          ReadRequest(
            name="Reactive Power",
            register=0x5B1C,
            size=2,
            convert=Client.to_hex,
          ),
          ReadRequest(
            name="Reactive Import",
            register=0x500C,
            size=2,
            convert=Client.to_uint32,
          ),
          ReadRequest(
            name="Reactive Export",
            register=0x5010,
            size=2,
            convert=Client.to_uint32,
          ),
          ReadRequest(
            name="Reactive Net",
            register=0x5014,
            size=2,
            convert=Client.to_sint32,
          ),
          ReadRequest(
            name="Active Import",
            register=0x5000,
            size=2,
            convert=Client.to_uint32,
          ),
          ReadRequest(
            name="Active Export",
            register=0x5004,
            size=2,
            convert=Client.to_uint32,
          ),
          ReadRequest(
            name="Active Net",
            register=0x5008,
            size=2,
            convert=Client.to_sint32,
          ),
          # WriteRequest(
          #   name="Tariff configuration", register=0x8C90, values=[0x0001]),
          # WriteRequest(name="Tariff daily", register=0x8A07, values=[0x0001]),
          # ReadRequest(
          #   name="Tariff configuration",
          #   register=0x8C90,
          #   size=1,
          #   convert=Client.to_hex,
          # ),
          # ReadRequest(
          #   name="Tariff",
          #   register=0x8A07,
          #   size=1,
          #   convert=Client.to_hex,
          # ),
          # WriteRequest(
          #   name="Tariff configuration", register=0x8C90, values=[0x0001]),
          # WriteRequest(name="Tariff nightly", register=0x8A07, values=[0x0002]),
          ReadRequest(
            name="Tariff configuration",
            register=0x8C90,
            size=1,
            convert=Client.to_hex,
          ),
          ReadRequest(
            name="Tariff",
            register=0x8A07,
            size=1,
            convert=Client.to_hex,
          ),
        ])

  if args.device_type() == DeviceType.schneider:
    while True:
      await execute(
        client,
        DeviceType.schneider,
        [
          ReadRequest(
            name="Model",
            register=0x0031,
            size=20,
            convert=Client.to_utf8,
          ),
          ReadRequest(
            name="Serial number",
            register=0x0081,
            size=2,
            convert=Client.to_uint32,
          ),
          ReadRequest(
            name="Active Power",
            register=0x0BF3,
            size=2,
            convert=Client.to_float32,
          ),
          ReadRequest(
            name="Active energy import total",
            register=0x0C83,
            size=4,
            convert=Client.to_sint64,
          ),
          ReadRequest(
            name="Active energy import L1",
            register=0x0DBD,
            size=4,
            convert=Client.to_sint64,
          ),
          ReadRequest(
            name="Active energy import L2",
            register=0x0DC1,
            size=4,
            convert=Client.to_sint64,
          ),
          ReadRequest(
            name="Active energy import L3",
            register=0x0DC5,
            size=4,
            convert=Client.to_sint64,
          ),
          # WriteRequest(name="Tariff configuration",
          #              register=5249,
          #              values=[2060, 0x0000, 0x0001]),
          # ReadRequest(
          #   name="Tariff configuration write result",
          #   register=5374,
          #   size=2,
          #   convert=Client.to_registers,
          # ),
          # WriteRequest(
          #   name="Tariff daily", register=5249, values=[2008, 0x0000, 0x0001]),
          # ReadRequest(
          #   name="Tariff write result",
          #   register=5374,
          #   size=2,
          #   convert=Client.to_registers,
          # ),
          # ReadRequest(
          #   name="Tariff",
          #   register=0x105E,
          #   size=1,
          #   convert=Client.to_hex,
          # ),
          # WriteRequest(name="Tariff configuration",
          #              register=5249,
          #              values=[2060, 0x0000, 0x0001]),
          # ReadRequest(
          #   name="Tariff configuration write result",
          #   register=5374,
          #   size=2,
          #   convert=Client.to_registers,
          # ),
          # WriteRequest(
          #   name="Tariff nightly", register=5249, values=[2008, 0x0000, 0x0002]),
          # ReadRequest(
          #   name="Tariff write result",
          #   register=5374,
          #   size=2,
          #   convert=Client.to_registers,
          # ),
          ReadRequest(
            name="Tariff",
            register=0x105E,
            size=1,
            convert=Client.to_hex,
          ),
        ])


async def execute(client: Client, device_type: DeviceType,
                  requests: List[Union[ReadRequest, WriteRequest]]):
  print("Executing requests for", device_type)
  start = time.time()
  for request in requests:
    if isinstance(request, ReadRequest):
      value = await client.read(
        register=request.register,
        count=request.size,
        convert=request.convert,
      )
      print("Read", request.name, value)
    else:
      await client.write(register=request.register, values=request.values)
      print("Wrote", request.name)
  end = time.time()
  print("took", end - start, "\n")


if __name__ == "__main__":
  from signal import SIGINT, SIGTERM

  async def wrapper():
    try:
      await main()
    except asyncio.CancelledError:
      pass

  def exception_handler(
    __loop__: asyncio.AbstractEventLoop,
    __ctx__: dict[str, Any],
  ):
    pass

  loop = asyncio.get_event_loop()
  main_task = asyncio.ensure_future(wrapper())
  loop.set_exception_handler(exception_handler)
  for signal in [SIGINT, SIGTERM]:
    loop.add_signal_handler(signal, main_task.cancel)
  try:
    loop.run_until_complete(main_task)
  finally:
    loop.close()
