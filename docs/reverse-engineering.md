# Original EA client compatibility investigation

Status: an initial native Linux implementation and a static reference audit. Full behavioral equivalence with the EA app has not been established.

The **Windows EA app is the primary reference for game behavior**: Linux runs Windows game downloads/installers/executables through Proton/umu. The supplied macOS client is used for shared interface-resource comparison, not as the game-package target. Current source explicitly requests the PC owned library (`isMac=false`, `platforms=[PC]`) and uses `JUNO_PC_CLIENT` for login. Download selection now retains the already queried offer `softwarePlatform`/`basePlatform` fields and requires explicit `PCWIN` evidence before queueing or requesting an archive URL. The `PCWIN` token is present in the supplied Windows executable and the existing commerce model; no new GraphQL argument was invented.

An explicit Windows LIVE build is preferred when returned build metadata provides it. Without build platform metadata, a single LIVE build is accepted only for an explicitly verified PCWIN offer; missing offer evidence, conflicting/non-Windows metadata, or ambiguous LIVE builds produce a clear error. The guard rechecks persisted/manual queue entries before even using a cached download URL. This conservative behavior can block downloads when the service omits the required evidence. It does not certify actual Win32/Win64 payload contents or compatibility; live Windows-download validation remains outstanding.

## Inspected reference build

The supplied Windows and macOS clients identify as **13.813.1.6327**. Windows `version.properties` reports `13.813.1-6327-d5ea2a1ad`.

| Input | SHA-256 |
| --- | --- |
| Windows `EADesktop.exe` | `cbf951f17a03d4c6a7a7e81ad538975625f52011625961832a9c8edee28d2807` |
| macOS `EA app.pkg` | `7460a57ad8748800f255a7c9e031329123419f838da12ce973bf4fcff212c435` |

The macOS installer contains 1,612 CPIO members and about 834 MB of regular file data. Its metadata declares x86_64 and arm64 hosts. Inspection extracted reference files with sanitized paths and did not execute the supplied client or installer scripts.

The Windows app imports Qt Quick, Qt WebChannel, Qt WebSockets and Chromium Embedded Framework (CEF). Windows Qt file metadata reports 5.15.2.0; macOS resources identify Qt 5.15.7. The CEF file resource identifies 109.1.18.0. These are observations about this supplied build, not current compatibility guarantees.

The inspected host is native Qt/QML with an embedded CEF renderer. An Electron application bundle was not identified in the inspected files. Treating this client as an ordinary Electron archive would miss the Qt-native configuration and bridge services.

## What the files reveal

Clear Qt resource bundles yield **526 resources** on each platform, including 125 QML views/components, six helper JavaScript files and six callback HTML pages. All 526 decoded resources match across the two platforms after CRLF normalization: 394 byte-identical files and 132 newline-only differences. The shared resources describe a native shell, menus, authentication callbacks, fonts, icons and the Quantum design system.

`shared/qml/SpaWebView.qml` selects the CEF window/item renderer and binds its URL to a native context value (`urlStr`). Binary metadata includes `app.spa.url`, `buildSpaUrl`, `initSpa` and `initWebProxies`. The main Store/Library SPA was **not found as a complete self-contained HTML/JS/CSS bundle in the clear resources inspected**. Its complete production URL and runtime payload have not been established by this static audit. Some other resource bundles are encrypted; they were not decrypted in this pass.

The observed web-channel wrapper uses Qt's `qt.webChannelTransport` or a local WebSocket at `ws://127.0.0.1:4695`. It discovers native objects and wraps their callbacks in promises. It supplies transport, not the implementation of account, download or game operations.

The follow-up inspection covered all 58 macOS Chromium DataPacks (437,373 entries), the Windows core Chromium packs, and selected native files/PE resources. It found no selected original main-SPA marker or reusable main frontend bundle within that inspected material. The production `app.spa.url` value remains unresolved; native host/path strings do not establish a complete URL. Opaque encrypted bundles remain outside the clear-resource audit, so this result does not claim the SPA is absent everywhere.

## Observed native bridge surface

The following identifiers occur in native metadata or clear callback resources. Their presence does not establish complete JSON schemas, timing or behavior.

| Original object | Observed methods/signals | Current Linux implementation |
| --- | --- | --- |
| Authentication | `getAuthentication`, `refreshAuthToken`, `logout` | aeapp OAuth flow is connected; account testing and original bridge parity remain unverified. |
| GamesManager | `getGameStatusV2`, `getBulkGameStatusV2`, `getInstallInfo`, `initiateDownload` | aeapp library/install/launch paths are connected. No original Qt WebChannel adapter is implemented. |
| GamesManager maintenance | `initiateUninstall`, `initiateUpdateV3`, `issueDownloadControl`, `resolveCloudSyncConflict` | Full maintenance/conflict UI parity remains incomplete. |
| GamesManager signals | `onInstallQueueChanged`, `onGameStatusUpdated`, `onGameLaunchStatusUpdated`, `onGameInstallError`, `onCloudSyncConflict` | aeapp events feed the current native UI; timing/schema parity is unverified. |
| Presence | `getFriendsPresence`, `updatePresence` | Current UI displays friends read only. Full presence editing is incomplete. |
| ClientSettings | `getLanguage`, `setLanguage`, `getInstallLocation`, `setInstallLocation` | Native frontend preferences and install directory are implemented independently. |
| OriginInfo / SpaRouting | SPA/bridge version metadata and native routing | No original-SPA host or full routing adapter. |
| Platform | `openFolder`, `getFreeDiskSpace` | Selected native operations exist; original platform object parity is incomplete. |

Original authentication callback pages invoke `authenticationJsHelper.onLoginSuccess(url)` and `onLogoutSuccess()`. aeapp's browser callback uses its own `aeapp-bootstrap` path. These are different implementations and must be tested as such.

## Visual reproduction

The current shell follows the supplied screenshots: a 40 px black titlebar, left EA navigation and installed games, central search/content, right profile/friends, account tabs and a separate per-game Proton panel. The Quantum reference provides primary blue `#276AFC`, secondary coral `#FF4747`, and navy surface values.

The actual OTF metadata identifies EA Text and Display regular faces as weight 400 and their genuine bold faces as weight 700. The shared QML avatar uses rounded squares (48 px with radius 12 for LG), and the clear resources contain exact small navigation SVGs and status masks. These are direct component measurements; they do not establish every CSS value in the separately delivered desktop SPA. Large settings/family/playtime illustrations were not located in the clear resources or selected Chromium packs. Substituting small control icons for those illustrations does not establish visual parity.

The repository contains independently authored interface code and fallback artwork. Optional EA Text/Display regular and bold fonts can be imported locally from user-provided `quantum.rcc` with `scripts/import-ea-fonts.py`; the same importer reads known static navigation SVGs from sibling `assets.rcc`. An original PNG icon may be supplied separately. Local covers and promotional artwork can be rendered where available. Imported assets and original EA executable/resource bundles are excluded from the source repository. Default builds retain the bundled fallback font and artwork.

The supplied Windows `EADesktop.exe` contains matching `MAINICON` and `IDI_ICON1` groups at 16, 32, 48, 64, 128 and 256 px. The largest icon is a byte-exact embedded PNG showing the original blue circle/white EA mark; it was extracted without resizing or re-encoding (SHA-256 `c1c0cac01c74eb7587422e134d84154fd720a5baa0f1828c9647fe435890216f`). It remains local. The earlier Mac rounded-square icon is a different platform asset, and is not evidence of the Windows icon's shape.

The native Home and Browse interfaces can display locally cached promotional and catalog assets. Original personalized promotional feeds, purchases, subscription management, social editing, notifications and other server-driven features do not have full native equivalents. Purchases remain in the external official EA flow. Neither preview games nor catalog entries establish account ownership.

Local artwork was obtained from public primary EA pages and retained with source URLs and hashes in local manifests. Examples include the [original Need for Speed Payback square cover](https://drop-assets.ea.com/images/6Qts38w7ndPeOoCa7SedfY/3da5f9e5b44c096a37a7bed030b0070c/NFSP_1x1.jpg), [Apex Blood Pact artwork](https://drop-assets.ea.com/images/21kMVLKxEenjwgtH5KZc67/8f8524ac4a8339c1692df46217b35de4/ApexS30_BloodPact_3840x2160.jpg), and [The Sims 4 wide art](https://app-images.ea.com/ps9x41qn6x3c/W633VThkiGHrEmK9peyoi/d18c48878fcbca44902ed1f25de09843/TS4_base-game-art-16x9_en_us.jpg). The recovered Sims image is official blue artwork with the white logo and ten Sims, but its cast differs from the supplied reference screenshot; it is not an exact recovered copy of that screenshot's hero asset. No synthesized artwork is presented as an original EA asset.

## Public storefront inspection

The [public catalog root](https://store.ea.com/commerce/app/products) and [Need for Speed Payback product page](https://store.ea.com/commerce/app/products/need-for-speed-payback) were statically inspected without authentication. The root response contained 161 server-rendered product cards on 2026-10-23; the product page contained VideoGame/Offer JSON-LD. The inspected pages expose names, official links, image URLs, exact price labels/currencies, and release years. They do not supply a full release date, ownership, subscription inclusion, or Linux compatibility for the imported cards.

These pages use their own inline catalog stylesheet and system font stack. The inspected material did not expose the supplied desktop component class, Qt WebChannel objects, original Home/Library/Settings component bundle, or source maps. A bounded download of the directly declared opaque script did not establish an original desktop bridge. This supports treating the public storefront as a separate catalog frontend; it does not establish that the desktop SPA is unavailable at another URL. Public catalog colors must not be presented as measurements of the desktop chrome.

`scripts/import-ea-catalog.py` offers a manual, standard-library import of up to 30 actual products and at most 20 JPEG/PNG/WebP images. HTTPS product and image URLs, including redirects, are restricted to known official hosts. It executes no scripts and invokes no authentication or checkout endpoint. Prices preserve the anonymous response currency (USD in this capture) and may change by region and time. The directly supplied catalog posters were portrait artwork, so square native cards crop them rather than inventing alternate image URLs.

The cache resides in local `ea-assets/catalog/products.json` with image paths relative to that directory. Preview rendering reads only local files. The user package and installer can copy the four fonts, selected icon, static SVGs, covers, promotions, catalog images, and known source JSON manifests through an explicit `--assets-dir`. Copying skips symlinks and unrelated files, is limited to 512 files/16 MiB per file/256 MiB total, and preserves existing assets. No account or payment data is included in catalog fixtures or the public source export.

## Work required before claiming 100% equivalence

1. Establish the main SPA payload/configuration in an authorized running reference session, or continue reproducing its visible behavior in native code.
2. Record method/property/signal schemas and failure behavior of each required native object, including account states, downloads, maintenance, cloud conflicts and social presence. Avoid retaining tokens or personal account data in fixtures.
3. Implement and test the corresponding aeapp-backed services or a native Qt/CEF compatibility host; a copied HTML page alone cannot supply the original native services.
4. Run account, entitlement, download, update, cloud-save and game-launch parity tests. Validate appearance at the reference viewport sizes and accessibility/input behavior.
5. Track game compatibility separately. Reproducing the launcher cannot by itself supply Windows kernel or anti-cheat support on Linux.

No live EA login, real game download, cloud-save write or real game launch was performed during this investigation. The current release is an experimental milestone toward the stated clone goal.

## Windows game-hub follow-up

The additional Windows screenshots guided the native Overview/Currency/Add-ons tab row, portrait game card, action placement, metadata column and background composition. The game actions reuse the existing download, launch, stop and settings handlers. Owned add-ons come from the account library; missing currency offers, achievements, release dates and age ratings remain explicitly unavailable. No screenshot account statistics or prices were copied as application data. The local Jedi preview now selects the official digital portrait pack art; a physical multi-platform cases image was rejected after visual inspection.


## Windows download options and membership badge follow-up

Screenshots 187–189 show a centered 600×484 Download options panel, 48px game thumbnail, install-path Change control, 40px language field, two install options, lower-left size/calculation status, and lower-right Next/Cancel controls. The native dialog follows this layout using Linux paths. Size comes from the guarded PCWIN archive central directory and free space from the destination filesystem; estimates use the full uncompressed archive, so resumed installs may be overestimated. Proton/runtime overhead is not included. Unsupported shortcut and automatic-update options remain disabled; the installer currently uses English. No real EA archive was fetched in verification. Local preview sizes are labeled demonstration fixtures through the Demo player account.

The user also requested the small white/red EA Play pill under the profile name. Its native rendering uses the locally imported EA medallion and text. The existing GetInitUserData query supplies subscription status; active subscriptions enable the badge, while unavailable or expired state does not. The response parser is tested offline. Live account membership has not been verified.


## Persistent download indicator follow-up

The user screenshot from 23 October 2026 at 16:50 shows a real active transfer in Downloads and no bottom-left shortcut. The Windows-style sidebar now reserves its final 100px for Download manager, using the locally imported DOWNLOAD_QUEUE symbol, the current game title, a progress ring and percentage. It reads the same current-offer snapshot as the full manager; stale snapshots belonging to another offer are ignored. Paused, failed, finalizing and unknown-size transfers keep truthful states. Clicking only changes the visible page. The installed-games scroll area already reserves 110px, so this remains clear at the supported 1080×700 minimum. Verification uses an isolated local transfer fixture; the user’s active process is not restarted.


## Runtime provisioning failure found during sidebar verification

The saved queue later contained `couldn’t find suitable wine release`. The transfer worker calls installer touchup after its archive entries have completed; that setup calls managed Proton provisioning. The selector required an unsuffixed `GE-ProtonN-N.tar.gz` name. The current [upstream GE-Proton11-11 release](https://github.com/GloriousEggroll/proton-ge-custom/releases/tag/GE-Proton11-11) instead lists separate `-aarch64.tar.gz` and `-x86_64.tar.gz` archives plus checksum files. The selector now prefers the host architecture, skips drafts/prereleases and incompatible releases, and only accepts the historical unsuffixed fallback on x86_64. Offline fixtures cover the actual upstream asset naming, wrong architectures, checksums, legacy archives, and unstable releases. No runtime archive was downloaded or executed in this verification, and the paused user queue was not resumed or modified.
