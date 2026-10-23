# Verification record

**Recorded: 23 October 2026.** This ledger separates the latest local UI milestone from the earlier, user-confirmed gameplay session. It reports observed results, rather than a compatibility guarantee.

[Project overview](../README.md) · [Linux setup and technical guide](../README-LINUX.md)

## Latest local milestone

| Check | Recorded result | Scope |
| --- | --- | --- |
| Automated tests | **76 passed, 0 failed** | 39 library, 35 UI, 2 browser callback tests |
| Live-account tests | **2 ignored** | Require a real account; excluded from the offline suite |
| Release build | **Passed** | `aeapp-ui`, `aeapp-bootstrap`, `aeapp-cli` |
| Isolated screenshot renders | **24 completed successfully** | 21 at 1920×1080; 3 at 1080×700 |
| Manual visual review | **14 representative renders inspected** | Includes failure, loading, enqueue, unknown-size and compact states |
| Binary integrity | **Matching SHA-256 values** | Source-built, packaged and locally installed binaries |

The recorded test command was:

```sh
cargo test --workspace --all-targets --locked --offline
```

The 35 UI tests were also rerun successfully after the final screenshot-input adjustment; they are not counted twice in the total above. The successful release build retained existing compiler warnings.

Regression coverage includes stale offer/path results, matching enqueue and control acknowledgments, retryable preparation errors, worker panics, constructor timeouts, truthful unavailable statistics, and window-close policy. The 60-second downloader deadline covers preparation before a transfer starts, rather than the duration of an active download.

The render set covers Home, Browse, Library, Profile, Settings, Game Hub, download options and Downloads. Fixtures include loading/error details, Currency/Add-ons tabs, insufficient space, pending/failed enqueue, paused/failed/finalizing downloads and unknown size. Screenshot mode ignores pointer input to keep hover tooltips out of the fixtures.

Representative repository images: [Home](screenshots/home.png), [Library](screenshots/library.png), [Game Hub](screenshots/game.png), [download options](screenshots/install.png), [Downloads](screenshots/downloads.png) and [Settings](screenshots/settings.png).

These images use demo account data and local preview fixtures. Displayed games, membership, progress and sizes are not evidence of account ownership or a real transfer. Preview mode does not start the EA backend, authenticate, launch a game or download games/runtimes.

## Earlier user-confirmed gameplay

On 23 October 2026, the user confirmed that **Need for Speed Payback played normally** through the native backend with managed **GE-Proton11-11** and an active **EA Play entitlement through Game Pass**. The real installation had completed, including its normal bundled prerequisite setup.

That observation concerns one game on the tested workstation. It predates this UI milestone and subsequent LSX-handler additions. The latest milestone did not repeat that gameplay session or start another real game/download; the earlier confirmation remains separate evidence.

## Limits of this verification

- The latest milestone performed no live-account actions and did not retest the actual desktop tray or a real Wayland compositor. Offline close-policy tests do not establish behavior on every desktop environment.
- Original EA frontend DOM and complete feature parity have not been recovered or established. This is an unofficial native client.
- Account playtime and achievement totals remain unavailable without a verified statistics source; unknown values must not appear as genuine zeroes.
- Non-English installer selection, game shortcut creation, automatic game-update policy and unsupported catalog filters remain disabled.
- Complete storefront checkout, DLC management and general game/anti-cheat compatibility are outside the verified scope. A game appearing in the library or preview is not a compatibility result.

The repository's [Linux CI workflow](../.github/workflows/build-ci.yml) defines automated test, release-build and isolated-preview checks. The local results above do not claim that a newly published commit has already passed GitHub Actions. This public ledger excludes private account data, license material, diagnostic account logs and workstation paths.
