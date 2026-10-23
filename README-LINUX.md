# EA for Linux · Maxima edition

Εγγενής Linux client βασισμένος στο [Maxima](https://github.com/ArmchairDevelopers/Maxima), με εμφάνιση εμπνευσμένη από την EA app και ρυθμίσεις Proton ανά παιχνίδι. Είναι ανεπίσημο community fork, χωρίς σύνδεση ή έγκριση από την Electronic Arts.

Το UI εκτελείται ως Linux binary. Τα υποστηριζόμενα Windows παιχνίδια ξεκινούν μέσω [umu](https://github.com/Open-Wine-Components/umu-launcher) και Proton. Η παρουσία ενός παιχνιδιού στη βιβλιοθήκη ή στο demo δεν αποτελεί επιβεβαίωση συμβατότητας.

## Γρήγορη εκκίνηση

Από τον φάκελο με τα binaries:

```sh
./maxima --preview
```

Το preview χρησιμοποιεί τοπικά δείγματα παιχνιδιών και ενδεικτικά γραφικά. Δεν εκκινεί το EA backend, δεν συνδέεται σε λογαριασμό, δεν κατεβάζει παιχνίδια ή runtimes και δεν ενεργοποιεί launch/install. Οι αλλαγές στις ρυθμίσεις του preview δεν αποθηκεύονται στις κανονικές ρυθμίσεις.

Διαθέσιμες αρχικές οθόνες:

```sh
./maxima --preview --preview-page library
./maxima --preview --preview-page home
./maxima --preview --preview-page settings
./maxima --preview --preview-page game
./maxima --preview --preview-page proton
```

Για PNG screenshot και αυτόματο κλείσιμο μετά την αποθήκευση:

```sh
./maxima --preview --preview-page library --screenshot /tmp/ea-linux-library.png
```

Απαιτείται ενεργό γραφικό περιβάλλον X11 ή Wayland. Ο φάκελος προορισμού του screenshot πρέπει να υπάρχει.

## Κανονική λειτουργία και σύνδεση EA

Κράτησε στον ίδιο φάκελο τα `maxima`, `maxima-bootstrap` και `maxima-cli`. Το `maxima-bootstrap` χρησιμοποιείται τόσο για την εκκίνηση παιχνιδιών όσο και για την επιστροφή από το browser κατά το OAuth login.

Για απευθείας εκκίνηση από source build:

```sh
mkdir -p "${XDG_DATA_HOME:-$HOME/.local/share}/applications"
./target/release/maxima
```

Το **Sign in with EA** ανοίγει το browser και χρησιμοποιεί το OAuth/PKCE flow του Maxima. Το desktop πρέπει να μπορεί να ανοίξει το `qrc:` protocol με το διπλανό `maxima-bootstrap`, που προωθεί την απάντηση στον τοπικό listener `127.0.0.1:31033`. Απαιτούνται system browser και `xdg-utils`. Κάνε την πρώτη κανονική εκκίνηση από την τελική θέση των binaries. Αν μετακινηθούν αργότερα, ο καταχωρισμένος protocol handler μπορεί να εξακολουθεί να δείχνει στην παλιά θέση.

Η κανονική λειτουργία χρησιμοποιεί το backend του Maxima για τα owned games, τη λήψη και την εκκίνηση. Το πρώτο launch ή import μπορεί να κατεβάσει αρκετά GB για GE-Proton και τα απαραίτητα runtimes.

## Ρυθμίσεις Proton ανά παιχνίδι

Άνοιξε ένα εγκατεστημένο παιχνίδι και επίλεξε **Manage game**:

- **Proton directory override:** άφησέ το κενό για managed GE-Proton ή δώσε απόλυτη τοπική διαδρομή προς εγκατάσταση Proton που περιέχει αρχείο `proton`, π.χ. `/home/player/.local/share/Steam/compatibilitytools.d/GE-Proton10-XX`.
- **Write Proton diagnostic log:** περνά `PROTON_LOG=1` στη διεργασία Proton.
- **Disable Esync / Disable Fsync:** περνούν αντίστοιχα `PROTON_NO_ESYNC=1` και `PROTON_NO_FSYNC=1`.

Η διαδρομή δίνεται ως directory, χωρίς shell commands ή πρόσθετα arguments. Όλα τα builds χρησιμοποιούν το κοινό prefix του Maxima. Η αλλαγή Proton build μπορεί να αναβαθμίσει αυτό το prefix· δεν δημιουργείται ξεχωριστό prefix ανά παιχνίδι.

Το **Settings → Linux & Proton** εμφανίζει την παρουσία των managed runtimes και τη θέση των δεδομένων.

## Δεδομένα στο Linux

Ο βασικός φάκελος είναι `${XDG_DATA_HOME}/maxima` ή, αν δεν ορίζεται το `XDG_DATA_HOME`, `~/.local/share/maxima`:

| Στοιχείο | Διαδρομή μέσα στον βασικό φάκελο |
| --- | --- |
| Managed GE-Proton | `wine/proton` |
| umu launcher | `wine/umu/umu-run` |
| Κοινό Wine prefix | `wine/prefix` |
| EAC runtime | `wine/eac_runtime` |
| Cache εικόνων | `cache/ui/images` |
| Αποθηκευμένο authentication | `auth.toml` |

Το `auth.toml` περιέχει στοιχεία συνεδρίας λογαριασμού και δεν πρέπει να συμπεριλαμβάνεται σε bug reports ή source archives. Το eframe UI χρησιμοποιεί app ID `io.github.maxima.EAForLinux` για τις δικές του ρυθμίσεις παραθύρου και interface.

## Build από τον πηγαίο κώδικα

Το `rust-toolchain.toml` ορίζει **Rust 1.93.0 stable**. Τα παλιά nightly feature gates έχουν αφαιρεθεί. Το `Cargo.lock` περιλαμβάνεται και το build γίνεται με `--locked`.

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
cargo build --locked --release -p maxima-ui -p maxima-bootstrap -p maxima-cli
```

Η προεπιλεγμένη έξοδος είναι `target/release/maxima`, `target/release/maxima-bootstrap` και `target/release/maxima-cli`. Τα κανονικά Cargo environment variables, όπως `CARGO_BUILD_JOBS` και `CARGO_TARGET_DIR`, εξακολουθούν να ισχύουν. Το πρώτο build χρειάζεται πρόσβαση στα crates και στα Git dependencies που ορίζει το workspace.

## Έλεγχοι και όρια

Καταγεγραμμένο αποτέλεσμα library tests: **6 passed, 2 ignored**. Οι δύο upstream cloud-save tests έχουν ρητό `ignore`, επειδή απαιτούν συνδεδεμένο EA account και live Battlefront II cloud saves· ο ένας μπορεί να γράψει σε αυτά. Δεν έχει πραγματοποιηθεί σύνδεση σε πραγματικό EA account ή επιβεβαίωση εκκίνησης πραγματικού παιχνιδιού.

```sh
cargo test --locked -p maxima-lib --lib
```

<!-- Final build and visual verification results are updated by the release author. -->
Local verification on Ubuntu 26.10 x86_64: release binaries built successfully with Rust 1.93.0; library, Home, account settings and Proton modal screenshots rendered under Xvfb. Six library tests passed, two live-account tests were ignored. The traced preview made no IPv4/IPv6 calls and launched no helper executable. Screenshots use locally imported EA fonts; the source build falls back to the bundled font. GitHub CI has not yet been verified.

Το upstream Maxima παραμένει πειραματικό. Storefront/αγορές, εγκατάσταση DLC και τα UI controls για stop/uninstall δεν αποτελούν ολοκληρωμένες λειτουργίες αυτής της έκδοσης. Η συμβατότητα διαφέρει ανά παιχνίδι, DRM και anti-cheat. Για τις απαιτήσεις του EA Javelin, δες την [επίσημη τεκμηρίωση EA](https://help.ea.com/en/articles/platforms/pc-ea-anticheat/). Η EA προτείνει έλεγχο της ένδειξης Steam Deck Compatibility στη σελίδα κάθε παιχνιδιού στο Steam: [EA download and play guidance](https://help.ea.com/en/articles/platforms/download-and-play-ea-app-games/).

## Πηγές και άδεια

- [Upstream Maxima](https://github.com/ArmchairDevelopers/Maxima)
- [umu launcher](https://github.com/Open-Wine-Components/umu-launcher)
- [GE-Proton](https://github.com/GloriousEggroll/proton-ge-custom)
- [GNU GPL v3 — τοπικό LICENSE](LICENSE)

Ο πηγαίος κώδικας αυτού του fork διανέμεται με **GPL-3.0**, όπως το Maxima. Τα ονόματα EA και παιχνιδιών ανήκουν στους αντίστοιχους δικαιούχους.

## Αυθεντικές τοπικές γραμματοσειρές

Μπορείς να εισάγεις EA Text/Display από δική σου εγκατάσταση EA Desktop χωρίς να εκτελέσεις τον Windows client:

```sh
python3 scripts/import-ea-fonts.py "/path/to/EA Desktop" "/path/to/EA-for-Linux/ea-assets"
```

Το native binary φορτώνει προαιρετικά τα `ea-text.otf` και `ea-display.otf` από τον διπλανό φάκελο `ea-assets`, ή από το `EA_LINUX_FONT_DIR`. Διαφορετικά χρησιμοποιεί τη γραμματοσειρά του Maxima. Τα εισαγμένα EA assets μένουν τοπικά και εξαιρούνται από το Git.

Η [αναφορά reverse engineering](docs/reverse-engineering.md) καταγράφει τι βρέθηκε στους Windows/Mac clients και τι απομένει για λειτουργική ισοδυναμία. Η έκδοση δεν είναι ακόμη επαληθευμένος 100% κλώνος.
