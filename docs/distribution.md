# Distributing the `ludi` CLI

Where the CLI binary should (and should not) be published, beyond the
GitHub release assets (`ludi-linux-x86_64`, `ludi-darwin-arm64`,
`ludi-windows-x86_64.exe`, and the matching `arm64`/`x86` variants). The
framework itself stays on LuaRocks; this is only about the CLI.

Status: plan agreed on 2026-07-07, not implemented yet.

## Planned channels (in order)

1. **Install script** — `curl -fsSL <site>/install.sh | sh`, the
   Go/Bun/Deno pattern. A static script that detects OS/arch and
   downloads the latest release binary into `/usr/local/bin` (or
   `~/.local/bin` without sudo). One file, zero maintenance, works on
   any Linux/macOS. Best effort/reach ratio — do this first.
2. **Homebrew tap** — repository `Ludi-Framework/homebrew-tap` with a
   formula pointing at the release binaries:
   `brew install ludi-framework/tap/ludi`. The release workflow bumps
   the formula (version + sha256) on every tag. homebrew-core proper
   requires notability; a tap does not.
3. **AUR** — `ludi-bin` (repackages the release binary; trivial
   PKGBUILD) and optionally `ludi` (builds from source with cargo).
   Publishing needs a maintainer account on aur.archlinux.org and an
   SSH key (as a repo secret for CI auto-bumps — e.g. the
   KSXGitHub/github-actions-deploy-aur action).
4. **Scoop bucket** — the Windows counterpart to the Homebrew tap:
   repository `Ludi-Framework/scoop-bucket` with a manifest pointing at
   the `ludi-windows-*.exe` assets (`scoop install ludi`). The release
   workflow bumps the manifest (version + hash) on every tag. The
   install script above covers the `curl | sh` crowd; this covers the
   Windows package-manager crowd.

## Rejected channels

- **LuaRocks** — a rock that compiles the CLI needs the Rust toolchain
  on the user's machine (defeats the point); a rock whose build step
  downloads a binary is against packaging expectations. Framework rock
  users can grab the binary from the release directly.
- **npm** — viable technically (esbuild/biome ship this way: one
  package per platform plus a launcher), but it assumes Node on a Lua
  developer's machine and costs several packages of upkeep. Revisit
  only if demand shows up.
