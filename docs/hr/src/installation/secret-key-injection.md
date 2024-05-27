# Umetanje tajnog ključa

Pomoću naredbe:

```bash
./inject <iso> <key>
```

ova skripta umeće tajni ključ u ISO sliku za specifični uređaj Raspberry Pi
koristeći skriptu `inject` iz repozitorija.

## Preduvjeti

Prije nego što započnete, provjerite jeste li generirali ISO sliku za uređaj
koristeći skriptu `image`. Skripta `inject` zahtijeva ISO sliku i datoteku s
tajnim ključem.

## Upotreba

Skripta `inject` uzima dva argumenta: `iso`, koji je put do ISO slike, i `key`,
koji je put do datoteke s tajnim ključem. Skripta provjerava postoje li te
datoteke. Ako ne postoje, izlazi s porukom o pogrešci.

Ovo je važno jer želimo da skripte koriste programi na uređaju koristeći `nix`,
što zahtijeva da tajni ključevi budu šifrirani u repozitoriju i dešifrirani na
uređaju prilikom pokretanja.
