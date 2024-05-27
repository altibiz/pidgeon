# Pidgeon

Pidgeon je aplikacija bazirana na Raspberry Pi-u dizajnirana za prikupljanje i
upravljanje podacima o električnom napajanju s različitih lokacija. To je
ključna komponenta u mreži za distribuciju električne energije, koja omogućuje
prikupljanje i prijenos podataka iz mjernih uređaja.

## Ključne značajke

- **Otkriće mjernih uređaja**: Pidgeon automatski otkriva mjerne uređaje koje
  prepoznaje na mreži lokacije.
- **Provjere stanja**: Redoviti pingovi i provjere stanja osiguravaju ispravno
  funkcioniranje mjernih uređaja i samog Pidgeona.
- **Prikupljanje podataka**: Pozadinski procesi započinju uzimanje električnih
  mjerenja visoke frekvencije kako bi osigurali točne i aktualne podatke.
- **Lokalno pohranjivanje**: Mjerenja se pohranjuju u lokalno instaliranu
  PostgreSQL bazu podataka, koja služi kao buffer prije slanja podataka na
  poslužitelj.
- **Komunikacija s poslužiteljem**: Pidgeon šalje mjerenja na poslužitelj i
  provjerava ima li ikakvih izmjena u konfiguraciji na poslužitelju.
- **Postavljanje tarifa**: Pidgeon je također odgovoran za postavljanje dnevnih
  i noćnih tarifa mjernih uređaja.

Optimiziranjem frekvencije mjerenja, Pidgeon osigurava najtočnije i najnovije
podatke. Ovi podaci su ključni za generiranje točnih informacija o naplati i
pružanje vrijednih podataka za istraživanje i analizu.
