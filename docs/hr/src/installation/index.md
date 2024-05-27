# Instalacija

Instalacija Pidgeona uključuje nekoliko koraka, od kojih je svaki detaljno
opisan na svojoj stranici. Evo pregleda procesa:

1. **Generiranje tajni**: Skripta u repozitoriju koristi `sops` i `openssl` za
   generiranje tajni za određeni Raspberry Pi. Ovaj korak je ključan za
   osiguravanje komunikacije između uređaja i poslužitelja.

2. **Kreiranje ISO slike**: Druga skripta u repozitoriju koristi `nix build` za
   kreiranje ISO slike za uređaj. Ova slika sadrži Pidgeon aplikaciju i sve
   njene ovisnosti.

3. **Umetanje tajnog ključa**: Tajni ključ generiran u koraku 1 umeće se u sliku
   pomoću skripte u repozitoriju. Tajni ključ se koristi za dešifriranje tajni
   generiranih u koraku 1 tijekom pokretanja.

4. **Sastavljanje uređaja**: ISO slika se zapisuje na 1TB SSD. SSD se zatim
   priključuje u USB port Raspberry Pi-a, a USB-C kabel za napajanje se
   priključuje.
