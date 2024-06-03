# Push

Prema [arhitekturi](../architecture.md), s gledišta Pidgeona, postoje višestruke
točke s kojih mogu nastati greške. Počevši od gatewaya pa sve do servera, možemo
očekivati greške u sljedećim područjima:

- **Gateway -> Pidgeon**: Gatewayi su odgovorni za prijenos mjerenja s mjerača
  na Pidgeon putem pull mehanizma. Postoji mali milijun načina na koji bi ovo
  moglo poći po zlu, ali s gledišta Pidgeona, relevantne su samo dvije
  kategorije scenarija: gateway ne šalje podatke ili gateway šalje netočne
  podatke.

- **Pidgeon**: Pidgeon sam po sebi može prestati raditi ili imati grešku koja
  uzrokuje prestanak povlačenja podataka s gatewaya.

- **Pidgeon -> Server**: Pidgeon šalje mjerenja na server. Ako je server
  isključen ili nema veze sa serverom ili postoje problemi s zahtjevima, server
  neće moći pohraniti podatke.

## Greške

Evo popisa grešaka koje se mogu dogoditi u push procesu podijeljenih po
područjima:

- **Gateway -> Pidgeon**:

  - Gateway ne šalje podatke
  - Gateway šalje netočne podatke

- **Pidgeon**:

  - Pidgeon nije povezan na mrežu
  - Pidgeon baca iznimku (softverska greška)

- **Pidgeon -> Server**:
  - Server nije povezan na mrežu
  - Server baca iznimku (softverska greška)

## Testiranje

Za testiranje otpornosti u push procesu, možemo simulirati greške na sljedeće
načine:

- Gateway ne šalje podatke: Isključite gateway i ponovno ga uključite. Pidgeon
  ne bi trebao biti pogođen.

- Gateway šalje netočne podatke: Promijenite podatke koje gateway šalje. Pidgeon
  bi trebao biti sposoban otkriti netočne podatke i ignorirati ih.

- Pidgeon nije povezan na mrežu: Isključite Pidgeon s mreže i ponovno ga
  spojite. Pidgeon bi trebao moći otkriti mrežni kvar i pokušati ponovno poslati
  podatke.

- Pidgeon baca iznimku: Uvedite grešku u Pidgeon koja uzrokuje da baci iznimku.
  Pidgeon bi trebao biti sposoban uhvatiti iznimku, zabilježiti je i nastaviti
  raditi.

- Server nije povezan na mrežu: Isključite server s mreže i ponovno ga spojite.
  Pidgeon bi trebao moći otkriti mrežni kvar i pokušati ponovno poslati podatke.

- Server baca iznimku: Uvedite grešku u server koja uzrokuje da baci iznimku.
  Pidgeon bi trebao biti sposoban uhvatiti iznimku, zabilježiti je i nastaviti
  raditi.
