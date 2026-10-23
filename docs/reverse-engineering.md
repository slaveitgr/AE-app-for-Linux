# Original EA client compatibility investigation

Status: an initial native Linux implementation and a static reference audit. Full behavioral equivalence with the EA app has not been established.

## Inspected reference build

The supplied Windows and macOS clients identify as **13.813.1.6327**. Windows `version.properties` reports `13.813.1-6327-d5ea2a1ad`.

| Input | SHA-256 |
| --- | --- |
| Windows `EADesktop.exe` | `cbf951f17a03d4c6a7a7e81ad538975625f52011625961832a9c8edee28d2807` |
| macOS `EA app.pkg` | `7460a57ad8748800f255a7c9e031329123419f838da12ce973bf4fcff212c435` |

The macOS installer contains 1,612 CPIO members and about 834 MB of regular file data. Its metadata declares x86_64 and arm64 hosts. Inspection extracted reference files with sanitized paths and did not execute the supplied client or installer scripts.

The Windows app imports Qt Quick, Qt WebChannel, Qt WebSockets and Chromium Embedded Framework (CEF). Windows Qt file metadata reports 5.15.2.0; macOS resources identify Qt 5.15.7. The CEF file resource identifies 109.1.18.0. These are observations about this supplied build, not current compatibility guarantees.

## What the files reveal

Clear Qt resource bundles yield **526 resources** on each platform, including 125 QML views/components, six helper JavaScript files and six callback HTML pages. All 526 decoded resources match across the two platforms after CRLF normalization: 394 byte-identical files and 132 newline-only differences. The shared resources describe a native shell, menus, authentication callbacks, fonts, icons and the Quantum design system.

`shared/qml/SpaWebView.qml` selects the CEF window/item renderer and binds its URL to a native context value (`urlStr`). Binary metadata includes `app.spa.url`, `buildSpaUrl`, `initSpa` and `initWebProxies`. The main Store/Library SPA was **not found as a complete self-contained HTML/JS/CSS bundle in the clear resources inspected**. Its complete production URL and runtime payload have not been established by this static audit. Some other resource bundles are encrypted; they were not decrypted in this pass.

The observed web-channel wrapper uses Qt's `qt.webChannelTransport` or a local WebSocket at `ws://127.0.0.1:4695`. It discovers native objects and wraps their callbacks in promises. It supplies transport, not the implementation of account, download or game operations.

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

The repository contains independently authored interface code and demo artwork. Optional EA Text/Display fonts can be imported locally from a user-provided `quantum.rcc` using `scripts/import-ea-fonts.py`. The imported fonts and original EA executable/resource bundles are not included in the source repository. Default builds retain the bundled aeapp font.

The Home screen currently presents library content. Original promotional feeds, purchases, subscription management, social editing, notifications and other server-driven features do not have full native equivalents. Browse and EA Play links open the official websites.

## Work required before claiming 100% equivalence

1. Establish the main SPA payload/configuration in an authorized running reference session, or continue reproducing its visible behavior in native code.
2. Record method/property/signal schemas and failure behavior of each required native object, including account states, downloads, maintenance, cloud conflicts and social presence. Avoid retaining tokens or personal account data in fixtures.
3. Implement and test the corresponding aeapp-backed services or a native Qt/CEF compatibility host; a copied HTML page alone cannot supply the original native services.
4. Run account, entitlement, download, update, cloud-save and game-launch parity tests. Validate appearance at the reference viewport sizes and accessibility/input behavior.
5. Track game compatibility separately. Reproducing the launcher cannot by itself supply Windows kernel or anti-cheat support on Linux.

No live EA login, real download, cloud-save write or real game launch was performed during this investigation. The current release is an experimental milestone toward the stated clone goal.
