# Push

According to the [architecture](../architecture.md), from the standpoint of the
Pidgeon, there are multiple points from which errors can occur. Starting from
the gateways and going to the server, we can expect to see errors in these
areas:

- **Gateway -> Pidgeon**: Gateways are responsible to transmit measurements from
  meters to Pidgeon via a pull mechanism. There are a little million ways that
  this could go wrong, but from the standpoint of Pidgeon, only two categories
  of scenarios are relevant, the gateway is not sending data or the gateway is
  sending incorrect data.

- **Pidgeon**: Pidgeon itself could stop working or have a bug that causes it to
  stop pulling data from gateways.

- **Pidgeon -> Server**: Pidgeon sends measurements to the server. If the server
  is down or there is no connection to the server or there are request problems,
  the server will not be able to store the data.

## Failures

Here is a list of failures that can occur in the push process divided into
areas:

- **Gateway -> Pidgeon**:

  - Gateway is not sending data
  - Gateway is sending incorrect data

- **Pidgeon**:

  - Pidgeon is not connected to the network
  - Pidgeon throws an exception (software bug)

- **Pidgeon -> Server**:
  - Server is not connected to the network
  - Server throws an exception (software bug)

## Testing

To test resiliency in the push process, we can simulate failures in the
following ways:

- Gateway is not sending data: Stop the gateway and start it back up. The
  Pidgeon should be unaffected.

- Gateway is sending incorrect data: Change the data that the gateway is
  sending. The Pidgeon should be able to detect the incorrect data and ignore
  it.

- Pidgeon is not connected to the network: Disconnect the Pidgeon from the
  network and reconnect it. The Pidgeon should be able to detect the network
  failure and retry sending the data.

- Pidgeon throws an exception: Introduce a bug in the Pidgeon that causes it to
  throw an exception. The Pidgeon should be able to catch the exception, log it
  and continue working.

- Server is not connected to the network: Disconnect the server from the network
  and reconnect it. The Pidgeon should be able to detect the network failure and
  retry sending the data.

- Server throws an exception: Introduce a bug in the server that causes it to
  throw an exception. The Pidgeon should be able to catch the exception, log it
  and continue working.
