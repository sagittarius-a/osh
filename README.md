# OSH - Opinionated Shell

OSH is a toy project to:

- Use Rust outside my comfort zone
- Figure out how complex it is to build a featureful shell
- Build the perfect KISS shell for me, eventually.
	- No buggy plugins 
	- No features that I will never use
	- Bare metal features only

As the name suggest, OSH is opinionated and may not fulfil your expectations. That's okay.

OSH is still in early development stage and full of bugs. Consider not using it. If you do, please enjoy.

## Features

- A single but classic prompt theme
- Completion (triggered with `TAB`) based on non-regex pattern. If several candidates are found, `skim` is used to filter them.
- Basic alias support including global alias: Alias can be anywhere in the command line
- Environment variable expansion
- Configuration can be edited thanks to the builtin `config` command. It uses `$EDITOR` as editor to open configuration file
- Completion hints based on history (like fish or zsh-auto-suggestions)
- Error code now available thanks to the `status` builtin command
- Prompt color changes based on error code
- Log feature is now functional
- Some additional keybindings have been implemented, such as:
	- `CTRL + f`: Accept completion hint
	- `CTRL + o`: Enter
	- `ALT + f`: Go forward a word
	- `ALT + w`: Go backward a word
	- `ALT + u`: Undo (That is soooooo cool)

## Notes

OSH is based on [bubble-shell](https://github.com/JoshMcguigan/bubble-shell).

## License

Licensed under either of

 * Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE.txt) or http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license ([LICENSE-MIT](LICENSE-MIT.txt) or http://opensource.org/licenses/MIT)

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the work by you,
as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.
