# Integration Spec: ez Share WiFi SD Card → ResMed AirSense 11 → AnxietyWatch2

Goal: read CPAP session data the AirSense 11 writes to the ez Share WiFi SD card, pull it over WiFi, parse the ResMed EDF files, and load them into PostgreSQL via a containerized polling service.

The card: ez Share full-size SDXC, Class 10, 64GB. SSID `ez Share`, WiFi password `88888888`, admin/config password `admin`.

---

## How it works, end to end

```
[ResMed AirSense 11] --writes EDF--> [ez Share WiFi SD @ 192.168.4.1, AP "ez Share"]
        │ (card WiFi is live while the CPAP is powered)
        ▼ WiFi AP  "ez Share" / 88888888
[Raspberry Pi / Router]          radio A → ez Share AP (192.168.4.x)
                                 radio B / eth → home LAN (192.168.x.x)
                                 forwards HTTP :80 → 192.168.4.1
        ▼ HTTP over LAN
[hms-cpap Service] --parse (cpap-py / pyedflib)--> [PostgreSQL (AnxietyWatch2)]
```

The ez Share exposes its files over plain HTTP. A background polling service walks the directory listing, downloads new files, parses the ResMed EDFs, and inserts physiological samples into the AnxietyWatch2 Postgres database.

---

## Step 1 — Format the card FAT32

The ez Share firmware reads **FAT32** only. A 64GB card ships as exFAT, so reformat it to FAT32 before use.

- **Linux:** `sudo mkfs.vfat -F 32 -n EZSHARE /dev/sdX1`.
- **macOS:** `diskutil eraseDisk FAT32 EZSHARE MBRFormat /dev/diskN`
- **Windows:** Use Rufus or `guiformat`.

Put the card in the AirSense 11 once and let it write `DATALOG/`.

---

## Step 2 — Confirm the card's HTTP API

Join the `ez Share` network from a laptop (password `88888888`) and verify the API:

Set `EZSHARE_HOST` to the card's documented access-point gateway, then query it:

- **Version:** `GET http://$EZSHARE_HOST/client?command=version`
- **Root listing:** `GET http://$EZSHARE_HOST/dir?dir=A:`
- **Download a file:** `GET http://$EZSHARE_HOST/download?file=<8.3_SHORTNAME>`

File listings are returned as HTML in a `<pre>` block, which must be parsed to extract the `download?file=` short name.

---

## Step 3 — Bridge the card's AP to your LAN

The card does not join your home WiFi; it hosts its own AP. To allow the `hms-cpap` service (running on the AnxietyWatch2 Raspberry Pi) to reach it without dropping off the LAN, use the Pi's networking:

- Pi onboard `wlan0` joins the `ez Share` AP (static route to 192.168.4.0/24).
- Pi Ethernet connects to your home LAN.
- The `hms-cpap` poller container is configured to talk through the `wlan0` interface to `192.168.4.1`.

---

## Step 4 — The `hms-cpap` poller

The ingest loop operates against the card's HTTP endpoints:

1. `GET /dir?dir=A:` to find the `DATALOG/` folder.
2. Maintain a "last synced date" cursor to prevent recursively parsing months of historical folders.
3. Download EDF files not previously processed (comparing the file name and KB size).
4. Parse the ResMed EDF formats. `cpap-py` (Python) is the recommended parser for AS11 data, handling BRP/PLD/SAD/EVE/CSL arrays and the daily `STR.edf`.
5. Map the parsed values to AnxietyWatch2 `vitals-registry` source/type enums.
6. Push immutable samples into the AnxietyWatch2 Postgres `samples` table. (No updates or deletes permitted).

---

## Troubleshooting

- **WiFi Not Visible:** The ez Share radio is only active while the CPAP supplies power.
- **Listing Returns Nothing:** Older ez Share firmware exposed only a photo-gallery API. Updating the card firmware restores the `/dir` capability.
- **Card Sees No Files:** If the CPAP wrote files but the HTTP endpoint is blank, confirm the card is formatted FAT32. ExFAT is invisible to the ez Share HTTP server.
