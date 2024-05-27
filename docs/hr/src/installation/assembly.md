# Sastavljanje

Ovo poglavlje opisuje završne korake procesa instalacije, koji uključuju
zapisivanje ISO slike na 1TB SSD i sastavljanje uređaja Raspberry Pi.

## Zapisivanje ISO slike

Da biste zapisali ISO sliku na SSD, možete koristiti naredbu `dd` na Linuxu ili
program kao što je Rufus na Windowsu.

### Linux

Na Linuxu možete koristiti naredbu dd za zapisivanje ISO slike na SSD. Prvo,
identificirajte put uređaja SSD-a pokretanjem naredbe lsblk. Kada dobijete put
uređaja, možete zapisati ISO sliku sljedećom naredbom:

```bash
sudo dd if=<iso> of=<device> bs=4M status=progress && sync
```

Zamijenite <iso> s putanjom do ISO slike i <device> s putanjom uređaja SSD-a.
Ova naredba zapisuje ISO sliku na SSD blok po blok i prikazuje informacije o
napretku. Naredba sync se koristi za osiguranje da su svi podaci ispravno
zapisani na uređaj.

### Windows

Na Windowsu možete koristiti program kao što je Rufus za zapisivanje ISO slike
na SSD. Evo koraka:

1. Preuzmite i instalirajte Rufus s službene web stranice.
2. Priključite SSD u USB port vašeg računala.
3. Otvorite Rufus i odaberite SSD u padajućem izborniku 'Device'.
4. U padajućem izborniku 'Boot selection' odaberite 'Disk or ISO image' i
   kliknite gumb 'Select' kako biste odabrali vaš ISO file.
5. Kliknite 'Start' za početak procesa. Rufus će formatirati SSD i zapisati ISO
   sliku na njega. Imajte na umu da će svi postojeći podaci na SSD-u biti
   izbrisani.

## Sastavljanje uređaja

Nakon zapisivanja ISO slike na SSD, možete sastaviti uređaj Raspberry Pi.

Odspojite SSD s računala. Priključite SSD u USB port Raspberry Pi-a. Priključite
USB-C kabel za napajanje kako biste uključili Raspberry Pi. Uređaj bi sada
trebao podići sustav s SSD-a i pokrenuti `pidgeon`.
