# Architecture

The architecture of Pidgeon is designed to efficiently collect and manage
electrical billing data. The diagram below provides a visual representation of
the system's architecture.

In the context of a location, there are various types of meters, such as the Abb
B2x meter and the Schneider iEM3xxx meter, which are connected via RS-485. The
Gateway, accessible via port 502, serves as an intermediary for data
communication.

The Raspberry Pi hosts the Pidgeon application, which is divided into three main
packages: Configuration, Services, and Processes.

- **Configuration**: This package contains the Manager component, responsible
  for managing the application's configuration.
- **Services**: This package contains several service components:
  - **Hardware**: Interacts with the physical hardware of the Raspberry Pi.
  - **Network**: Manages network communications.
  - **Modbus**: Handles the Modbus protocol for communication with the meters.
  - **Database**: Manages the local PostgreSQL database.
  - **Cloud**: Handles communication with the cloud server.
- **Processes**: This package contains various processes that Pidgeon runs:
  - **Discovery**: Discovers meters on the network.
  - **Ping**: Regularly checks the health of the meters.
  - **Measure**: Takes electrical measurements from the meters.
  - **Health**: Checks the health of Pidgeon and stores it in the local
    database.
  - **Push**: Sends measurements to the cloud server.
  - **Poll**: Polls the cloud server for configuration updates.
  - **Update**: Updates the server of meter and Raspberry PI health.
  - **Daily**: Sets the daily tariff of the meters.
  - **Nightly**: Sets the nightly tariff of the meters.

Please refer to the diagram for a visual representation of these components and
their interactions.

```plantuml
@startuml

left to right direction

cloud "Location" {
  node "Abb B2x meter" as abb {
    portin "RS-485" as abb_rs485
  }

  node "Schneider iEM3xxx meter" as schneider {
    portin "RS-485" as schneider_rs485
  }

  node "Gateway" as gateway {
    portin "502" as gateway_502
  }

  node "Raspberry PI" as rpi {
    package "Pidgeon" as pidgeon {
      package "Configuration" as configuration {
        component Manager as config_manager
      }

      package Services as services {
        component Hardware as hardware_service
        component Network as network_service
        component Modbus as modbus_service
        component Database as database_service
        component Cloud as cloud_service
      }

      package Processes as processes {
        component Discovery as discovery_process
        component Ping as ping_process
        component Measure as measure_process
        component Push as push_process
        component Poll as poll_process
        component Update as update_process
        component Health as health_process
        component Daily as daily_process
        component Nightly as nightly_process
      }
    }

    database "PostgreSQL" as postgres {
      portin 5432 as postgres_5432
    }
  }
}

cloud "Azure" as azure {
  node "Server" as server {
    portin "/iot/push" as server_push
    portin "/iot/poll" as server_poll
    portin "/iot/update" as server_update
  }
}

gateway --> abb_rs485 : "Modbus RTU"
gateway --> schneider_rs485 : "Modbus RTU"

config_manager --> rpi : "File system"

modbus_service --> gateway_502 : "Modbus TCP"
database_service --> postgres_5432 : "SQL"
hardware_service --> rpi : "File system"
network_service --> gateway_502 : "TCP"
cloud_service --> server_push : "HTTP"
cloud_service --> server_poll : "HTTP"
cloud_service --> server_update : "HTTP"

processes ..> services : use

@enduml
```
