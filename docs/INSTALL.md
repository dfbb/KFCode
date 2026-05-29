# Installing KFCode

KFCode is distributed as a prebuilt `kfcode` binary. Pick whichever method fits your
platform. Homebrew is the simplest on macOS and Linux; Windows users get a PowerShell
installer.

## Supported platforms

| OS | Architecture | Method |
|----|--------------|--------|
| macOS | Apple Silicon (arm64) | Homebrew, shell installer, manual |
| Linux | x86_64 (amd64) | Homebrew, shell installer, manual |
| Windows | x86_64 (amd64) | PowerShell installer, manual |

> **Not supported:** Intel Macs (`x86_64`), 32-bit systems, Linux on ARM. On these,
> Homebrew and the installers will fail to find a matching build.

## Homebrew (macOS arm64 / Linux amd64)

```bash
brew install dfbb/tap/kfcode
```

That command taps `dfbb/homebrew-tap` and installs `kfcode` in one step. To tap
explicitly first:

```bash
brew tap dfbb/tap
brew install kfcode
```

## Shell installer (macOS / Linux)

```bash
curl --proto '=https' --tlsv1.2 -LsSf \
  https://github.com/dfbb/KFCode/releases/latest/download/kfcode-cli-installer.sh | sh
```

The script downloads the right archive for your platform and installs `kfcode` into
`$CARGO_HOME/bin` (defaults to `~/.cargo/bin`). Make sure that directory is on your
`PATH` — see [PATH setup](#path-setup).

## PowerShell installer (Windows)

```powershell
irm https://github.com/dfbb/KFCode/releases/latest/download/kfcode-cli-installer.ps1 | iex
```

Installs `kfcode.exe` into `%CARGO_HOME%\bin` (defaults to `%USERPROFILE%\.cargo\bin`)
and adds it to your user `PATH`. Open a new terminal afterwards so the updated `PATH`
takes effect.

## Manual download

Grab the archive for your platform from the
[latest release](https://github.com/dfbb/KFCode/releases/latest):

| Platform | Archive |
|----------|---------|
| macOS arm64 | `kfcode-cli-aarch64-apple-darwin.tar.gz` |
| Linux amd64 | `kfcode-cli-x86_64-unknown-linux-gnu.tar.gz` |
| Windows amd64 | `kfcode-cli-x86_64-pc-windows-msvc.zip` |

The binary sits in a platform-named subdirectory inside the archive, e.g.
`kfcode-cli-aarch64-apple-darwin/kfcode`.

**macOS / Linux:**

```bash
tar -xzf kfcode-cli-aarch64-apple-darwin.tar.gz
# move the binary somewhere on your PATH
sudo mv kfcode-cli-aarch64-apple-darwin/kfcode /usr/local/bin/
```

**Windows:** extract the `.zip` and move `kfcode.exe` into a directory that is on your
`PATH`.

### Verifying the download

Every archive ships with a `.sha256` checksum. Verify before extracting:

```bash
# macOS / Linux
shasum -a 256 -c kfcode-cli-aarch64-apple-darwin.tar.gz.sha256
```

```powershell
# Windows: compare the printed hash against the .sha256 file
Get-FileHash kfcode-cli-x86_64-pc-windows-msvc.zip -Algorithm SHA256
```

## PATH setup

The installers place the binary in the Cargo bin directory. If `kfcode` is not found
after installing, add that directory to your `PATH`.

```bash
# macOS / Linux — add to ~/.zshrc or ~/.bashrc
export PATH="$HOME/.cargo/bin:$PATH"
```

```powershell
# Windows (PowerShell, current user) — then restart the terminal
[Environment]::SetEnvironmentVariable(
  "Path", "$env:USERPROFILE\.cargo\bin;" + [Environment]::GetEnvironmentVariable("Path","User"), "User")
```

Homebrew installs into its own prefix, which is already on your `PATH`, so this step is
not needed when installing via `brew`.

## Verify the installation

```bash
kfcode --help
```

## Upgrading

```bash
# Homebrew
brew update && brew upgrade kfcode

# shell / PowerShell installer — re-run the same one-liner to get the latest
```

Manual installs: download the newer archive and replace the binary.

## Uninstalling

```bash
# Homebrew
brew uninstall kfcode
brew untap dfbb/tap   # optional: remove the tap

# installer / manual — delete the binary
rm ~/.cargo/bin/kfcode          # macOS / Linux
# Windows: delete %USERPROFILE%\.cargo\bin\kfcode.exe
```

## Troubleshooting

- **`kfcode: command not found`** — the binary's directory is not on your `PATH`. See
  [PATH setup](#path-setup), then open a new terminal.
- **macOS "cannot be opened because the developer cannot be verified"** — the binary is
  not notarized. Remove the quarantine attribute:
  `xattr -d com.apple.quarantine $(which kfcode)`. Prefer Homebrew, which avoids this.
- **`brew install` reports no available formula / bottle** — you are likely on an
  unsupported platform (e.g. Intel Mac). See [Supported platforms](#supported-platforms).

## See also

- Build & release internals: `docs/BUILD.md`
- User guide: `USER_GUIDE.md`

