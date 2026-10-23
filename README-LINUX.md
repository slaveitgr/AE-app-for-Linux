# aeapp για Linux

Εγγενής Linux client για παιχνίδια EA, με εμφάνιση εμπνευσμένη από την EA app και ρυθμίσεις Proton ανά παιχνίδι. Είναι ανεπίσημο community fork, χωρίς σύνδεση ή έγκριση από την Electronic Arts.

Το UI εκτελείται ως Linux binary. Τα υποστηριζόμενα Windows παιχνίδια ξεκινούν μέσω [umu](https://github.com/Open-Wine-Components/umu-launcher) και Proton. Η παρουσία ενός παιχνιδιού στη βιβλιοθήκη ή στο demo δεν αποτελεί επιβεβαίωση συμβατότητας.

## Γρήγορη εκκίνηση

Από τον φάκελο με τα binaries:

```sh
./aeapp --preview
```

Το preview χρησιμοποιεί τοπικά δείγματα παιχνιδιών και ενδεικτικά γραφικά. Δεν εκκινεί το EA backend, δεν συνδέεται σε λογαριασμό, δεν κατεβάζει παιχνίδια ή runtimes και δεν ενεργοποιεί launch/install. Οι αλλαγές στις ρυθμίσεις του preview δεν αποθηκεύονται στις κανονικές ρυθμίσεις.

Διαθέσιμες αρχικές οθόνες:

```sh
./aeapp --preview --preview-page library
./aeapp --preview --preview-page home
./aeapp --preview --preview-page browse
./aeapp --preview --preview-page ea-play
./aeapp --preview --preview-page profile
./aeapp --preview --preview-page downloads
./aeapp --preview --preview-page settings
./aeapp --preview --preview-page game
./aeapp --preview --preview-page proton
```

Για PNG screenshot και αυτόματο κλείσιμο μετά την αποθήκευση:

```sh
./aeapp --preview --preview-page library --screenshot /tmp/ea-linux-library.png
```

Απαιτείται ενεργό γραφικό περιβάλλον X11 ή Wayland. Ο φάκελος προορισμού του screenshot πρέπει να υπάρχει.

## Κανονική λειτουργία και σύνδεση EA

Κράτησε στον ίδιο φάκελο τα `aeapp`, `aeapp-bootstrap` και `aeapp-cli`. Το `aeapp-bootstrap` χρησιμοποιείται τόσο για την εκκίνηση παιχνιδιών όσο και για την επιστροφή από το browser κατά το OAuth login.

Για απευθείας εκκίνηση από source build:

```sh
mkdir -p "${XDG_DATA_HOME:-$HOME/.local/share}/applications"
./target/release/aeapp
```

Το **Sign in with EA** ανοίγει το browser και χρησιμοποιεί το OAuth/PKCE flow του aeapp. Το desktop πρέπει να μπορεί να ανοίξει το `qrc:` protocol με το διπλανό `aeapp-bootstrap`, που προωθεί την απάντηση στον τοπικό listener `127.0.0.1:31033`. Απαιτούνται system browser και `xdg-utils`. Κάνε την πρώτη κανονική εκκίνηση από την τελική θέση των binaries. Αν μετακινηθούν αργότερα, ο καταχωρισμένος protocol handler μπορεί να εξακολουθεί να δείχνει στην παλιά θέση.

Η κανονική λειτουργία χρησιμοποιεί το backend του aeapp για τα owned games, τη λήψη και την εκκίνηση. Το πρώτο launch ή import μπορεί να κατεβάσει αρκετά GB για GE-Proton και τα απαραίτητα runtimes.

## Λήψεις και χειρισμός παιχνιδιών

Στο **Downloads** μπορείς να κάνεις **Pause/Resume** της ενεργής λήψης και **Cancel**, που σταματά τη μεταφορά και διατηρεί τα ήδη κατεβασμένα αρχεία. Το **Download next** μεταφέρει ένα παιχνίδι στην αρχή της λίστας αναμονής, ενώ το **Remove** αφαιρεί μια εγγραφή που περιμένει.

Η σειρά και η κατάσταση της ουράς αποθηκεύονται στο `download_queue.json` μέσα στον βασικό φάκελο δεδομένων του aeapp. Μετά από επανεκκίνηση της εφαρμογής, η ουρά επανέρχεται σε παύση και συνεχίζει όταν επιλέξεις **Resume**. Τα ολοκληρωμένα αρχεία ελέγχονται ξανά· τα ημιτελή αρχεία ξεκινούν από την αρχή μετά από επανεκκίνηση της διεργασίας. Το Pause/Resume μέσα στην ίδια εκτέλεση της εφαρμογής διατηρεί τη μεταφορά στη μνήμη.

Το **Stop** ζητά τερματισμό μόνο από Linux διεργασίες παιχνιδιού με το ακριβές launch marker της εφαρμογής και διαδρομή που αντιστοιχεί στον φάκελο του παιχνιδιού. Εξαιρούνται οι κοινές βοηθητικές διεργασίες Wine και το bootstrap. Η κατάσταση του ενεργού παιχνιδιού διατηρείται μέχρι να ολοκληρωθεί η κανονική διαχείριση εξόδου και cloud saves. Το **Open game folder** ανοίγει τον φάκελο ενός εγκατεστημένου παιχνιδιού. Το **Refresh library** ανανεώνει ρητά τα στοιχεία ιδιοκτησίας και εγκατάστασης, παρακάμπτοντας το διάστημα της ownership cache.

## Ρυθμίσεις Proton ανά παιχνίδι

Άνοιξε ένα εγκατεστημένο παιχνίδι και επίλεξε **Manage game**:

- **Proton directory override:** άφησέ το κενό για managed GE-Proton ή δώσε απόλυτη τοπική διαδρομή προς εγκατάσταση Proton που περιέχει αρχείο `proton`, π.χ. `/home/player/.local/share/Steam/compatibilitytools.d/GE-Proton10-XX`.
- **Write Proton diagnostic log:** περνά `PROTON_LOG=1` στη διεργασία Proton.
- **Disable Esync / Disable Fsync:** περνούν αντίστοιχα `PROTON_NO_ESYNC=1` και `PROTON_NO_FSYNC=1`.

Η διαδρομή δίνεται ως directory, χωρίς shell commands ή πρόσθετα arguments. Όλα τα builds χρησιμοποιούν το κοινό prefix του aeapp. Η αλλαγή Proton build μπορεί να αναβαθμίσει αυτό το prefix· δεν δημιουργείται ξεχωριστό prefix ανά παιχνίδι.

Το **Settings → Linux & Proton** εμφανίζει την παρουσία των managed runtimes και τη θέση των δεδομένων.

## Δεδομένα στο Linux

Ο βασικός φάκελος είναι `${XDG_DATA_HOME}/aeapp` ή, αν δεν ορίζεται το `XDG_DATA_HOME`, `~/.local/share/aeapp`:

| Στοιχείο | Διαδρομή μέσα στον βασικό φάκελο |
| --- | --- |
| Managed GE-Proton | `wine/proton` |
| umu launcher | `wine/umu/umu-run` |
| Κοινό Wine prefix | `wine/prefix` |
| EAC runtime | `wine/eac_runtime` |
| Cache εικόνων | `cache/ui/images` |
| Αποθηκευμένο authentication | `auth.toml` |
| Ουρά λήψεων | `download_queue.json` |

Το `auth.toml` περιέχει στοιχεία συνεδρίας λογαριασμού και δεν πρέπει να συμπεριλαμβάνεται σε bug reports ή source archives. Το eframe UI χρησιμοποιεί app ID `io.github.slaveitgr.aeapp` για τις δικές του ρυθμίσεις παραθύρου και interface.

## Build από τον πηγαίο κώδικα

Το `rust-toolchain.toml` ορίζει **Rust 1.93.0 stable**. Το `Cargo.lock` περιλαμβάνεται και το build γίνεται με `--locked`.

Απαιτούνται Rust μέσω rustup, C/C++ build tools, `pkg-config` και **`protobuf-compiler` / `protoc`**. Σε Debian/Ubuntu, τα βασικά build prerequisites είναι:

```sh
sudo apt-get install build-essential pkg-config protobuf-compiler
```

Ανάλογα με τη διανομή και το desktop, μπορεί να χρειάζονται επίσης development packages για X11, Wayland, xkbcommon και OpenGL/EGL. Για εκτέλεση απαιτούνται οι αντίστοιχες βιβλιοθήκες γραφικών και κατάλληλος GPU driver.

Από τη ρίζα του repository:

```sh
./scripts/build-linux.sh
```

Το script εκτελεί:

```sh
cargo build --locked --release -p aeapp-ui -p aeapp-bootstrap -p aeapp-cli
```

Η προεπιλεγμένη έξοδος είναι `target/release/aeapp`, `target/release/aeapp-bootstrap` και `target/release/aeapp-cli`. Τα κανονικά Cargo environment variables, όπως `CARGO_BUILD_JOBS` και `CARGO_TARGET_DIR`, εξακολουθούν να ισχύουν. Το πρώτο build χρειάζεται πρόσβαση στα crates και στα Git dependencies που ορίζει το workspace.

## Εγκατάσταση για τον χρήστη

Μετά το build:

```sh
./scripts/install-linux.sh --from target/release
```

Αν έχεις αποσυμπιέσει το binary package, εκτέλεσε `./install-linux.sh` μέσα στον φάκελό του. Δεν απαιτείται root. Το script εγκαθιστά τα binaries στο `${XDG_DATA_HOME:-$HOME/.local/share}/ae-app-for-linux`, δημιουργεί την εντολή `~/.local/bin/aeapp` και προσθέτει το **aeapp** στο μενού εφαρμογών. Η επανεκτέλεσή του ενημερώνει τα binaries και διατηρεί τα δεδομένα λογαριασμού/παιχνιδιών και τα ήδη εισαγμένα τοπικά assets.

Για δικές σου τοπικές γραμματοσειρές και το αυθεντικό εικονίδιο PNG:

```sh
./scripts/install-linux.sh --from target/release \
  --assets-dir /path/to/local/ea-assets \
  --icon /path/to/local/ea-icon.png
```

Ο φάκελος assets μπορεί να περιέχει τις EA Text/Display σε regular και bold, το `icon.png`, στατικά `icons/*.svg` και τοπικά γραφικά στους φακέλους `covers`, `promotions` και `catalog`. Αντιγράφονται μόνο αυτά και τα γνωστά JSON πηγών, χωρίς symlinks, αρχεία λογαριασμού ή cache. Τα υπάρχοντα τοπικά assets διατηρούνται. Το επιλεγμένο εικονίδιο αντιγράφεται χωρίς αλλαγή μεγέθους. Τα εισαγμένα EA assets εξαιρούνται από το Git· ένα νέο προεπιλεγμένο package περιλαμβάνει fallback assets. Ο εγκατεστημένος wrapper ορίζει `AEAPP_INSTALLED=1` και το script καταχωρίζει τον κρυφό handler `aeapp-qrc.desktop` για το browser callback. Η εγκατάσταση δεν ανοίγει την εφαρμογή και δεν πραγματοποιεί σύνδεση σε λογαριασμό.

Για package με τα δικά σου τοπικά assets:

```sh
./scripts/package-linux.sh /path/to/local/package --assets-dir /path/to/local/ea-assets
```

## Έλεγχοι και όρια

Καταγεγραμμένοι έλεγχοι: **25 library tests, 2 UI tests και 2 browser callback tests — 29 passed συνολικά, με 2 library tests ignored**. Οι δύο upstream cloud-save tests έχουν ρητό `ignore`, επειδή απαιτούν συνδεδεμένο EA account και live Battlefront II cloud saves· ο ένας μπορεί να γράψει σε αυτά. Δεν έχει πραγματοποιηθεί σύνδεση σε πραγματικό EA account ή επιβεβαίωση εκκίνησης πραγματικού παιχνιδιού.

```sh
cargo test --locked -p aeapp-lib --lib
cargo test --locked -p aeapp-ui --bin aeapp
cargo test --locked -p aeapp-bootstrap --bin aeapp-bootstrap
```

<!-- Final build and visual verification results are updated by the release author. -->
Local verification on Ubuntu 26.10 x86_64: all three release binaries built with Rust 1.93.0; 41 tests passed and two live-account tests were ignored. Home, Browse, EA Play, profile, Library, Downloads, the five original settings tabs, and Proton previews were visually checked. Native tray registration, X-to-hide with the process still alive, tray-to-restore, Settings dispatch, and Exit were exercised against the desktop tray host with isolated test data. The icon now uses the exact PNG embedded in the supplied Windows EADesktop.exe. User installation, desktop-file validation, visible menu entry, and a real Gio `qrc:` callback round trip to a local fixture passed without an EA account. The installed icon matches the exact Windows MAINICON PNG (`c1c0cac01c74eb7587422e134d84154fd720a5baa0f1828c9647fe435890216f`). X11 inspection confirmed the borderless window, title `aeapp`, and WM_CLASS `io.github.slaveitgr.aeapp`. The traced preview made no IPv4/IPv6 calls and launched no helper executable. Screenshots use locally imported EA fonts; the source build uses the bundled fallback. GitHub [Linux build and preview](https://github.com/slaveitgr/AE-app-for-Linux/actions/runs/39945970498) passed for the latest published baseline commit `d3c8e7a`; the current changes have separate local verification.

Το aeapp παραμένει πειραματικό. Storefront/αγορές, εγκατάσταση DLC και απεγκατάσταση παιχνιδιού από το UI δεν αποτελούν ολοκληρωμένες λειτουργίες αυτής της έκδοσης. Η συμβατότητα διαφέρει ανά παιχνίδι, DRM και anti-cheat. Για τις απαιτήσεις του EA Javelin, δες την [επίσημη τεκμηρίωση EA](https://help.ea.com/en/articles/platforms/pc-ea-anticheat/). Η EA προτείνει έλεγχο της ένδειξης Steam Deck Compatibility στη σελίδα κάθε παιχνιδιού στο Steam: [EA download and play guidance](https://help.ea.com/en/articles/platforms/download-and-play-ea-app-games/).

## Πηγές και άδεια

- [Αναφορές συντελεστών](CREDITS.md)
- [umu launcher](https://github.com/Open-Wine-Components/umu-launcher)
- [GE-Proton](https://github.com/GloriousEggroll/proton-ge-custom)
- [GNU GPL v3 — τοπικό LICENSE](LICENSE)

Ο πηγαίος κώδικας διανέμεται με **GPL-3.0**. Τα ονόματα EA και παιχνιδιών ανήκουν στους αντίστοιχους δικαιούχους.

## Αυθεντικές τοπικές γραμματοσειρές

Μπορείς να εισάγεις EA Text/Display από δική σου εγκατάσταση EA Desktop χωρίς να εκτελέσεις τον Windows client:

```sh
python3 scripts/import-ea-fonts.py "/path/to/EA Desktop" "/path/to/AEApp-for-Linux/ea-assets"
```

Το script εισάγει `ea-text.otf`, `ea-text-bold.otf`, `ea-display.otf`, `ea-display-bold.otf` και γνωστά στατικά SVG από το διπλανό `assets.rcc`. Το native binary φορτώνει αυτά και προαιρετικό `icon.png` από τον διπλανό φάκελο `ea-assets`. Το `AEAPP_ASSET_DIR` ορίζει εναλλακτικό φάκελο, ενώ το παλιότερο `AEAPP_FONT_DIR` εξακολουθεί να υποστηρίζεται. Διαφορετικά χρησιμοποιεί τα fallback assets του aeapp. Τα εισαγμένα EA assets μένουν τοπικά και εξαιρούνται από το Git.

Ο [δημόσιος κατάλογος της EA](https://store.ea.com/commerce/app/products) μπορεί να εισαχθεί χειροκίνητα για το native Browse:

```sh
python3 scripts/import-ea-catalog.py /path/to/local/ea-assets/catalog
```

Αποθηκεύονται έως 30 πραγματικά προϊόντα και 20 επίσημες εικόνες. Οι τιμές κρατούν το νόμισμα της ανώνυμης σελίδας και μπορεί να αλλάζουν ανά περιοχή και χρόνο. Το preview διαβάζει μόνο την τοπική εισαγωγή, χωρίς αυτόματη λήψη. Οι αγορές ανοίγουν την επίσημη εξωτερική ροή της EA. Η εμφάνιση στον κατάλογο δεν δηλώνει ιδιοκτησία, συνδρομητική πρόσβαση ή συμβατότητα Linux.

Η [αναφορά reverse engineering](docs/reverse-engineering.md) καταγράφει τι βρέθηκε στους Windows/Mac clients και στις δημόσιες σελίδες. Δεν ανακτήθηκε το αρχικό desktop SPA· ο δημόσιος κατάλογος είναι διαφορετικό server-rendered frontend. Η έκδοση δεν είναι ακόμη επαληθευμένος 100% κλώνος.

## Εικονίδιο tray

Σε Linux με διαθέσιμο StatusNotifierItem/AppIndicator host, το tray χρησιμοποιεί το εικονίδιο της εγκατάστασης και προσφέρει Open aeapp, Settings και Exit. Το Χ κρύβει το παράθυρο μόνο μετά από επιτυχή εγγραφή στο tray. Αν χαθεί το tray, κρυμμένο παράθυρο επανέρχεται. Χωρίς host το Χ τερματίζει κανονικά. Το preview δεν συνδέεται στο D-Bus και δεν δημιουργεί tray.

Η τρέχουσα υλοποίηση παραθύρου χρησιμοποιεί X11/XWayland όταν υπάρχει `DISPLAY`, επειδή το native Wayland backend της δεν υποστηρίζει απόκρυψη. Το close-to-tray σε καθαρό Wayland χωρίς XWayland δεν έχει επαληθευτεί.

## Game Hub των Windows

Η οθόνη παιχνιδιού ακολουθεί τα Windows screenshots με Overview, Currency και Add-ons, εξώφυλλο, κάρτα παιχνιδιού και Download/Play/Stop. Το μενού παιχνιδιού διατηρεί Proton settings και άνοιγμα φακέλου. Τα add-ons χρησιμοποιούν την πραγματική βιβλιοθήκη, ενώ στοιχεία που δεν φορτώθηκαν δεν εμφανίζονται ως εικονικά στατιστικά, ημερομηνίες ή τιμές. Για τοπικό έλεγχο: `--preview --preview-page game --preview-game-tab 0` (ή 1/2).


### Windows download options and account badge

The 600×484 download dialog follows the Windows app, with an editable Linux game folder, English installer language, disk-space calculation, insufficient-space state, and the existing Locate installed game action. Next requires a verified Windows build and enough free space for the full uncompressed archive; this is checked again before queuing. The estimate is conservative for resumed installs and excludes extra Proton/runtime space. Desktop shortcut creation and automatic game updates remain visibly unavailable. Preflight runs independently of active downloads.

The EA Play badge appears below the account name when the existing EA subscription query reports an active subscription. Unknown, failed, and expired membership results do not grant a badge. Local previews use an explicitly sample badge. Preview the dialog with `--preview --preview-page install --preview-install-state ready` (also `calculating`, `insufficient`, or `error`).


Κάτω αριστερά υπάρχει μόνιμο Download manager. Σε ενεργή λήψη εμφανίζονται το παιχνίδι, η πραγματική πρόοδος και δακτύλιος προόδου, με αντίστοιχη ένδειξη παύσης/σφάλματος/ολοκλήρωσης εγκατάστασης. Το κλικ ανοίγει μόνο το Downloads. Τοπική προεπισκόπηση: `--preview --preview-page home --preview-active-download`. Η ενημέρωση των binaries γίνεται ατομικά και μπορεί να εγκατασταθεί ενώ η παλιά διεργασία συνεχίζει τη λήψη· η νέα διεπαφή ενεργοποιείται στην επόμενη πλήρη εκκίνηση.


Η επιλογή managed GE-Proton υποστηρίζει τα σύγχρονα archives `-x86_64.tar.gz` και `-aarch64.tar.gz`, επιλέγοντας τη σωστή αρχιτεκτονική του υπολογιστή. Τα παλιότερα archives χωρίς κατάληξη αρχιτεκτονικής γίνονται δεκτά μόνο για x86_64. Draft/prerelease εκδόσεις και αρχεία checksum αγνοούνται. Αυτό διορθώνει το σφάλμα `couldn’t find suitable wine release` που μπορούσε να σταματήσει την τελική προετοιμασία μετά τη λήψη. Η ήδη αποθηκευμένη ουρά συνεχίζεται με Resume/Retry.
