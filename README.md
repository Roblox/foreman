# Foreman
Foreman is a toolchain manager to help Roblox developers manage their installations of tools like [Rojo](https://github.com/rojo-rbx/rojo), [Remodel](https://github.com/rojo-rbx/remodel), [Tarmac](https://github.com/rojo-rbx/tarmac), and [Selene](https://github.com/Kampfkarren/selene).

Foreman is inspired by [rustup](https://rustup.rs) and [asdf](https://github.com/asdf-vm/asdf).

It's an early prototype, but feedback at this stage is welcome!

## Installation
You can download pre-built Foreman releases for Windows, macOS, and Linux from the [Releases](https://github.com/rojo-rbx/foreman/releases) page.

Alternatively, you can install from the `master` branch if you have [Rust](https://www.rust-lang.org/) 1.41.0 or newer installed:

```bash
cargo install --git https://github.com/rojo-rbx/foreman.git
```

On first run (try `foreman list`), Foreman will create a `.foreman` directory in your user folder (`~/.foreman` on Unix systems, `%USERPROFILE%/.foreman` on Windows).

It's recommended that you **add `~/.foreman/bin` to your `PATH`** to make the tools that Foreman installs for you accessible on your system.

## Usage
Foreman downloads tools from GitHub and references them by their `user/repo` name, like `rojo-rbx/foreman`.

### System Tools
To start using Foreman to manage your system's default tools, create the file `~/.foreman/foreman.toml`.

A Foreman config that lists Rojo could look like:

```toml
[tools]
rojo = { source = "rojo-rbx/rojo", version = "0.5.0" }
```

Run `foreman install` from any directory to have Foreman pick up and install the tools listed in your system's Foreman config.

Now, if you run `rojo` inside of a directory that doesn't specify its own version of Rojo, Foreman will run the most recent 0.5.x release for you!

### Project Tools
Managing a project's tools with Foreman is similar to managing system tools. Just create a `foreman.toml` file in the root of your project.

A Foreman config that lists Remodel might look like this:

```toml
[tools]
remodel = { source = "rojo-rbx/remodel", version = "0.6.1" }
```

Run `foreman install` to tell Foreman to install any new binaries from this config file.

When inside this directory, the `remodel` command will run the latest 0.6.x release of Remodel installed on your system.

### Authentication
To install tools from a private GitHub repository, Foreman supports authenticating with a [Personal Access Token](https://help.github.com/en/github/authenticating-to-github/creating-a-personal-access-token-for-the-command-line).

Open `~/.foreman/auth.toml` after Foreman has run at least once to see instructions on how to set this up.

## Troubleshooting
Foreman is a work in progress tool and has some known issues. Check out [the issue tracker](https://github.com/rojo-rbx/foreman/issues) for known bugs.

If you have issues with configuration, you can delete `~/.foreman` to delete all cached data and start from scratch. This directory contains all of Foreman's installed tools and configuration.

## License
Foreman is available under the MIT license. See [LICENSE.txt](LICENSE.txt) or <https://opensource.org/licenses/MIT> for details.