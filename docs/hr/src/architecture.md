# Arhitektura

Arhitektura Pidgeona dizajnirana je za učinkovito prikupljanje i upravljanje
podacima o električnom napajanju. Dijagram ispod pruža vizualni prikaz
arhitekture sustava.

U kontekstu jedne lokacije postoje različiti tipovi brojila, kao što su Abb B2x
brojilo i Schneider iEM3xxx brojilo, koji su povezani putem RS-485. Gateway,
dostupan putem porta 502, služi kao posrednik za komunikaciju podataka.

Raspberry Pi pokreće Pidgeon aplikaciju, koja je podijeljena na tri glavna
paketa: Konfiguracija, Usluge i Procesi.

- **Konfiguracija**: Ovaj paket sadrži komponentu Manager, odgovornu za
  upravljanje konfiguracijom aplikacije.
- **Usluge**: Ovaj paket sadrži nekoliko servisnih komponenti:
  - **Hardware**: Interagira s fizičkim hardverom Raspberry Pi-a.
  - **Network**: Upravljanje mrežnim komunikacijama.
  - **Modbus**: Upravljanje Modbus protokolom za komunikaciju s mjeriteljima.
  - **Database**: Upravljanje lokalnom PostgreSQL bazom podataka.
  - **Cloud**: Upravljanje komunikacijom s cloud serverom.
- **Procesi**: Ovaj paket sadrži različite procese koje Pidgeon pokreće:
  - **Discovery**: Otkriva brojila na mreži.
  - **Ping**: Redovito provjerava stanje brojila.
  - **Measure**: Preuzima električna mjerenja s brojila.
  - **Health**: Provjerava stanje Pidgeona i pohranjuje ga u lokalnu bazu
    podataka.
  - **Push**: Šalje mjerenja na cloud server.
  - **Poll**: Provjerava cloud server za ažuriranja konfiguracije.
  - **Update**: Ažurira server o stanju brojila i Raspberry PI-a.
  - **Daily**: Postavlja dnevnu tarifu brojila.
  - **Nightly**: Postavlja noćnu tarifu brojila.

Dijagram za vizualni prikaz ovih komponenti i njihovih interakcija:

```plantuml
@startuml

left to right direction

cloud "Lokacija" {
  node "Mjeritelj" as meter {
    portin "RS-485" as meter_rs485
  }

  node "Raspberry PI" as rpi {
    component "Datotečni sustav" as filesystem

    package "Pidgeon" as pidgeon {
      package "Konfiguracija" as configuration {
        component Manager as config_manager
      }

      package Usluge as services {
        component Hardware as hardware_service
        component Network as network_service
        component Modbus as modbus_service
        component Database as database_service
        component Cloud as cloud_service
      }

      package Procesi as processes {
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

    database "Relacijska baza podataka" as database {
      portin 5432 as dbPort
    }
  }
}

cloud "Cloud" as cloud {
  node "Server" as server {
    portin "/iot/push" as server_push
    portin "/iot/poll" as server_poll
    portin "/iot/update" as server_update
  }
}

config_manager --> filesystem

modbus_service --> meter_rs485 : "Modbus RTU"
database_service --> dbPort : "SQL"
hardware_service --> filesystem
cloud_service --> server_push : "HTTP"
cloud_service --> server_poll : "HTTP"
cloud_service --> server_update : "HTTP"

processes ..> services : koristi

@enduml
```
