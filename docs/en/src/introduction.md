# Pidgeon

Pidgeon is a Raspberry Pi-based application designed to fetch and manage
electrical billing data from various sites. It's a crucial component in an
electrical distribution network, facilitating the collection and transmission of
meter data.

## Key Features

- **Meter Discovery**: Pidgeon automatically discovers meters it recognizes on
  the site's network.
- **Health Checks**: Regular pings and health checks ensure the meters and
  Pidgeon itself are functioning correctly.
- **Data Collection**: Workers are started to take electrical measurements at a
  high frequency to ensure accurate and up-to-date data.
- **Local Storage**: Measurements are stored in a locally installed PostgreSQL
  database, serving as an outbox before the data is sent to the server.
- **Server Communication**: Pidgeon sends the measurements to the server and
  polls the server for any edited configuration.
- **Tariff Setting**: Pidgeon is also responsible for setting the daily and
  nightly tariffs of the meters.

By optimizing for the frequency of measurement, Pidgeon ensures the most
accurate and current data is always available. This data is crucial for
generating accurate billing information and providing valuable data for research
and analysis.
