# Foreman
Foreman will be a toolchain manager to help Roblox developers manage their installations of tools like [Rojo](https://github.com/rojo-rbx/rojo), [Remodel](https://github.com/rojo-rbx/remodel), [Tarmac](https://github.com/rojo-rbx/tarmac), and [Selene](https://github.com/Kampfkarren/selene).

Foreman is inspired by [rustup](https://rustup.rs) and [asdf](https://github.com/asdf-vm/asdf).

There's nothing usable yet, but check back soon!

## Usage
To start, Foreman will be a tiny tool to download binaries from GitHub releases.

Users will add tools globally using a command like:

```bash
foreman install rojo-rbx/rojo
```

Foreman will download the latest release of Rojo and put it into Foreman's internal tool storage, like `~/.foreman/tools/rojo-rbx/rojo/rojo-0.6.0-alpha.1`.

Foreman will also create a symlink back to itself in a path like `~/.foreman/bin/rojo`. Foreman will assume that `~/.foreman/bin` is on the user's `PATH` environment variable.

Running `rojo` at this point should act just like running Rojo itself would.

When Foreman is run and its executable name is set to something other than `foreman`, it'll search for a configuration file like `foreman.toml`, which users can use to configure what versions of various tools to use.

A project's `foreman.toml` file might look like this:

```toml
[tools]
rojo = "rojo-rbx/rojo@0.5.0"
remodel = "rojo-rbx/remodel@0.4.0"
selene = "kampfkarren/selene@=1.0.0"
```

Foreman will use these version ranges to ensure a compatible version of a given tool (by SemVer range) is installed. Versions specified in the project will override the versions of these tools installed on the user's machine globally.

To install all the tools for a given project explicitly, users will be able to run:

```bash
foreman install
```

Foreman will warn the user if a project uses the same binary name but has a different source.

This command is not necessary if the user already has any version of any of the listed tools installed.