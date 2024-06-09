# clap-hot-reload

Add hot-reload capabilities to any Rust [CLAP](https://github.com/free-audio/clap) plugin!

This is a small library that wraps a traditional CLAP entry structure. When loaded by a host, it spawns a file
watcher thread, and reloads all the contained plugins whenever the CLAP bundle changes.

The reloading happens on the fly, reloading the plugin's GUI and parameters (among other things) and switching audio
processors without interruption, even while the DAW is processing audio or the transport is running.

This library is based on [Clack](https://github.com/prokopyl/clack) for CLAP host and plugin integration.

## State of development

This project is in its very early stage, quite unfinished and probably not that robust, although it works great on
my computer™.

### Development next steps / future ideas

- [ ] Finish implementing support for all CLAP extensions
- [ ] Add hot-reload capabilities to e.g. audio ports, note ports, etc.
- [ ] Harden the file watcher against e.g. symlink loops and other fun stuff that can happen on filesystems
- [ ] Write documentation and publish on crates.io
- [ ] Expose a C API, so it can be used by non-Rust projects

## License

`clap-hot-reload` is distributed under the terms of both the [MIT license](LICENSE-MIT) and
the [Apache license, version 2.0](LICENSE-APACHE).
Contributions are accepted under the same terms.
