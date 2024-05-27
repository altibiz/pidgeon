# Generiranje tajni

Pomoću naredbe:

```bash
scripts/mksecrets <cloud_domain> <network_ip_range_start> <network_ip_range_end>
```

, ova skripta generira set tajni za specifični uređaj Raspberry Pi koristeći
OpenSSL i SOPS, te ih priprema za umetanje u ISO sliku.

## Koraci

1. Skripta uzima tri argumenta: `cloud_domain`, `network_ip_range_start` i
   `network_ip_range_end`. Provjerava jesu li ti argumenti osigurani, inače
   izlazi s porukom o pogrešci.

2. Postavlja direktorije za pohranu tajni i privremenih tajni.

3. Generira jedinstveni ID za uređaj i provjerava postoje li već tajne za taj
   ID. Ako postoje, izlazi s porukom o pogrešci.

4. Definira nekoliko pomoćnih funkcija za generiranje različitih vrsta tajni
   (ID-ovi, ključevi, lozinke, age ključevi, SSH ključevi, SSL certifikati). Te
   tajne se generiraju koristeći OpenSSL, age-keygen, ssh-keygen i mkpasswd.

5. Generira tajne za različite komponente (altibiz, api, pidgeon, secrets,
   postgres) koristeći te pomoćne funkcije.

6. Stvara PostgreSQL skriptu za postavljanje baze podataka i korisnika s
   njihovim pripadajućim lozinkama.

7. Stvara datoteku okruženja (pidgeon.env) s raznim postavkama konfiguracije,
   uključujući URL baze podataka, cloud domenu, API ključ, mrežni IP raspon itd.

8. Stvara YAML datoteku (secrets.yaml) s generiranim tajnama.

9. Šifrira datoteku secrets.yaml koristeći SOPS i javne age ključeve altibiz,
   pidgeon i secrets. Šifrirana datoteka (secrets.enc.yaml) zatim se kopira u
   direktorij src/flake/enc s jedinstvenim ID-om uređaja kao njenim imenom.

Nakon što skripta završi, generirane tajne mogu se umetnuti u ISO sliku za
uređaj Raspberry Pi.
