# Contributing

Thank you for considering making contributions to the Noir project!

## Overview

Individuals making significant and valuable contributions are given commit access to the project. Contributions are done via pull requests and need to be approved by the maintainers.

## Rules

Here are some basic rules for all contributors (including the maintainers):

- **No `--force` pushes** or modifying the main branch history in any way. If you need to rebase, make sure to do it in your own repository. Do not rewrite the history after the code has been shared (e.g., through a Pull Request).
- **Non-main branches**, prefixed with a short name identifier (e.g., `conr2d/my-feature`), must be used for ongoing work.
- **All modifications** must be made in a **pull request** to solicit feedback from other contributors.
- A pull request **must not be merged until CI** has completed successfully.
- A pull request must pass tests with `cargo clippy && cargo fmt && cargo test`.

## Contributor License Agreement

All contributors must sign the Contributor License Agreement (CLA) before any contributions can be accepted. The signing process is automated via CLA Assistant, allowing you to agree through comments while submitting your PR.
