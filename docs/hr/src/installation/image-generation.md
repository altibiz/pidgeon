# Generiranje slike

Pomoću naredbe:

```bash
scripts/image <id>
```

, ova skripta generira ISO sliku za specifični uređaj Raspberry Pi koristeći
skriptu `image` iz repozitorija.

## Preduvjeti

Prije nego što započnete, provjerite jeste li generirali tajne ključeve za
uređaj koristeći skriptu `mksecrets`. Skripta `image` zahtijeva šifriranu
datoteku s tajnim ključevima za uređaj.

## Upotreba

Skripta `image` uzima jedan argument: `id`, koji je jedinstveni identifikator za
uređaj. Skripta provjerava postoji li šifrirana datoteka s tajnim ključevima za
ovaj ID u direktoriju `src/flake/enc`. Ako ne postoji, skripta izlazi s porukom
o pogrešci.
