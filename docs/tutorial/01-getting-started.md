# Getting Started

## Installation

Riven ships as a prebuilt toolchain for Linux and macOS. The installer drops
everything under `~/.riven` and adds `~/.riven/bin` to your `PATH` via your
shell rc file.

### One-line install

```bash
curl -fsSL https://raw.githubusercontent.com/sherazp995/riven/master/install.sh | bash
```

Pick up the new `PATH` in the current shell:

```bash
source "$HOME/.riven/env"
```

Or open a new terminal. Confirm that it worked:

```bash
riven --version
rivenc --version
```

### What gets installed

```
~/.riven/
  bin/
    riven          # package manager & build tool
    rivenc         # standalone compiler (and formatter)
    riven-lsp      # LSP server for editors
    riven-repl     # interactive REPL
  lib/
    runtime.c      # C runtime source (used at link time)
  env              # shell snippet that adds bin/ to PATH
  version          # installed release tag
```

### Install options

```bash
# Pin a specific release
curl -fsSL https://raw.githubusercontent.com/sherazp995/riven/master/install.sh \
  | bash -s -- --version v0.1.0

# Don't touch shell rc files
curl -fsSL https://raw.githubusercontent.com/sherazp995/riven/master/install.sh \
  | bash -s -- --no-modify-path

# Install somewhere other than ~/.riven
RIVEN_HOME=/opt/riven curl -fsSL \
  https://raw.githubusercontent.com/sherazp995/riven/master/install.sh | bash
```

### Uninstalling

```bash
curl -fsSL https://raw.githubusercontent.com/sherazp995/riven/master/uninstall.sh | bash
```

Or manually: remove `~/.riven` and delete the `. "$HOME/.riven/env"` line from
your shell rc file.

### Upgrading

Re-run the installer. It overwrites the binaries in `~/.riven/bin` and bumps
`~/.riven/version`.

## Your First Program

Create a file called `hello.rvn`:

```riven
puts "Hello, Riven!"
```

Compile and run:

```bash
rivenc hello.rvn
./hello
```

You should see:

```
Hello, Riven!
```

## Creating a Project

For anything beyond a single file, use the package manager:

```bash
riven new my_app
cd my_app
```

This creates:

```
my_app/
  Riven.toml        # project manifest
  src/
    main.rvn        # entry point
```

Build and run with:

```bash
riven build
riven run
```

## Project Commands

| Command | What it does |
|---------|--------------|
| `riven new <name>` | Create a new project |
| `riven init` | Initialize a project in the current directory |
| `riven build` | Compile the project (incremental) |
| `riven run` | Build and run |
| `riven check` | Type-check without producing a binary |
| `riven clean` | Remove build artifacts |
| `riven clean --global` | Clear the global cache at `~/.cache/riven/` |
| `riven add <dep>` | Add a dependency |
| `riven remove <dep>` | Remove a dependency |
| `riven update` | Refresh the lockfile |
| `riven tree` | Show the dependency graph |

## The REPL

Fire up an interactive session:

```bash
riven-repl
```

```
Riven 0.1.0 REPL — Type :help for commands
> 1 + 2
=> 3 : Int
> let x = "world"
> "hello #{x}"
=> "hello world" : String
> :type 1.0 + 2.0
Float
> :quit
```

REPL commands: `:help`, `:type <expr>`, `:reset`, `:quit`.

## Compiler Flags

```bash
rivenc hello.rvn              # compile with Cranelift (fast)
rivenc --release hello.rvn    # compile with LLVM (optimized, requires LLVM 18)
rivenc -o mybin hello.rvn     # custom output name
rivenc --emit=ast hello.rvn   # inspect AST (also: tokens, hir, mir)
rivenc --force hello.rvn      # ignore incremental cache, rebuild from scratch
rivenc --verbose hello.rvn    # log cache hits/misses
rivenc fmt hello.rvn          # format in place
rivenc fmt --check .          # check formatting without changes
rivenc fmt --diff file.rvn    # show a unified diff
```

## Editor Support

Install the VSCode extension from `editors/vscode/` in the Riven repo for
syntax highlighting, hover info, go-to-definition, and error diagnostics. The
extension launches `riven-lsp` from your `PATH`, so no further configuration
is needed after installation.

## Troubleshooting

**`riven: command not found` after installing.** Your current shell hasn't
picked up the new `PATH`. Either run `source "$HOME/.riven/env"` or open a new
terminal.

**Installer can't resolve the latest release.** GitHub is rate-limiting
unauthenticated requests. Pin a version: `... | bash -s -- --version v0.1.0`.

**Install script downloaded but won't run on macOS.** macOS Gatekeeper may
quarantine the binaries. Run:
`xattr -dr com.apple.quarantine "$HOME/.riven/bin"`.

**Need to reset everything.** Remove `~/.riven`, remove `~/.cache/riven`, and
delete the `. "$HOME/.riven/env"` line from your shell rc file.
