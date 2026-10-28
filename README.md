<p align="center">
  <img src="packaging/icon.svg" width="96" height="96" alt="Mincord icon">
</p>

<h1 align="center">Mincord</h1>

<p align="center">
  <strong>A lightweight, native Discord client.</strong><br>
  Built with Rust and Slint, with a focus on performance and a responsive desktop experience.
</p>

<p align="center">
  <a href="https://github.com/mincord/mincord/releases/latest">Latest release</a>
  ·
  <a href="https://github.com/mincord/mincord/issues">Report a bug</a>
  ·
  <a href="https://mincord.com/discord-invite">Community</a>
</p>

<p align="center">
  <a href="https://github.com/mincord/mincord/releases/latest"><img src="https://img.shields.io/github/v/release/mincord/mincord?label=release&color=5865F2" alt="Latest release"></a>
  <a href="LICENSE"><img src="https://img.shields.io/github/license/mincord/mincord" alt="License"></a>
</p>

About
Mincord is a desktop client project exploring a more lightweight way to use Discord without embedding a full browser engine. It is written in Rust and uses Slint for its interface.

The aim is to keep the application responsive, reduce idle resource usage, and make everyday messaging and community features available in a native desktop environment.

Mincord is under active development. Feature coverage and platform support may change between releases.
Highlights
- Native desktop interface built with Rust and Slint.
- Performance-focused design intended to reduce idle memory and CPU overhead.
- Messaging and communities, including servers, channels, direct messages and group conversations.
- Voice and media features, including calls, camera support and screen sharing where supported.
- Rich conversations with replies, reactions, attachments, embeds, stickers and Markdown.
- Community tools for roles, channel configuration and moderation where permissions allow.
- Personalisation through themes and appearance settings.
- Local caching to make previously loaded content available quickly.
- Open source with a buildable development workflow.
Availability varies by platform and release. Check the release notes and project documentation for the current implementation status.
Platform support
Platform	Availability
Windows 10 / 11	Release builds available
Linux	Release builds available for supported environments
macOS	Not currently supported by the project


Linux package requirements can vary by distribution. Refer to the release assets and build documentation before installing.
Installation
Windows
1. Open the latest releases.
2. Download the Windows installer or portable archive.
3. Run the installer, or extract the archive and launch the application.
4. Follow the in-app instructions to connect your account.
Windows may display a security warning if the installer is not signed. Only run software you have reviewed and obtained from a source you trust.
Linux
1. Visit the latest releases.
2. Choose the package that matches your distribution and system.
3. Follow the instructions for that package type, then launch Mincord.
4. Complete the in-app account connection flow.
For package-specific requirements and alternative installation methods, see the project documentation.
Build from source
Mincord uses a Nix development environment to provide its expected toolchain and native dependencies.
# Run the UI preview without connecting an account
nix develop path:./nix --command cargo run --locked -- --preview

# Run the desktop client
nix develop path:./nix --command cargo run --locked

# Build the Discord-only configuration
nix develop path:./nix --command cargo run --locked --no-default-features --features desktop,discord

# Run formatting, linting and tests
scripts/check --fast
Build commands and required features may evolve. Consult the documentation before making changes to the build process.
Documentation
Guide	Description
[Development](docs/development.md)	Development workflow and local testing
[Architecture](docs/architecture.md)	Overview of the client and core components
[Building](docs/building.md)	Toolchain, build options and release packaging
[Discord implementation](docs/discord/README.md)	Implemented functionality and planned work
[Voice](docs/voice.md)	Voice and audio implementation details
[Windows](docs/windows-beta.md)	Windows packaging and platform notes
[Measurements](docs/measurements/README.md)	Performance measurement methodology


Project status
Mincord is a work in progress. Some Discord features may be incomplete, behave differently from the official client, or not be supported yet. Review the issue tracker and documentation for known limitations, and please report reproducible problems with enough detail to investigate them.
- Browse open issues
- Review releases
- Contribute or suggest an improvement
Privacy and account safety
Mincord communicates with Discord services directly from the application. Review the source code and relevant documentation to understand how credentials, cached content, media and network requests are handled.

Mincord is an unofficial third-party client and is not affiliated with or endorsed by Discord. Use of third-party clients may carry account-related risks and may be inconsistent with Discord's terms or policies. Review Discord's current terms before using one, and do not use the application with an account you cannot afford to lose.
Frequently asked questions
<details>
<summary><strong>Does Mincord use a browser engine?</strong></summary>

Mincord is designed around a native Rust and Slint interface rather than embedding a full Chromium-based application shell.
</details>

<details>
<summary><strong>Can I use plugins from other Discord clients?</strong></summary>


Compatibility with plugins made for other clients is not assumed. Those plugins may rely on different runtimes, internal APIs or interface structures.
</details>

<details>
<summary><strong>Is Mincord affiliated with Discord?</strong></summary>


No. Mincord is an independent, unofficial project. Discord is a trademark of Discord Inc.
</details>

<details>
<summary><strong>Where can I report an issue?</strong></summary>


Please open an issue in the GitHub issue tracker. Include your operating system, application version, steps to reproduce the problem and any relevant logs, while removing tokens or other sensitive information.
</details>

License
Mincord is distributed under the GNU General Public License, version 3 or later. Some build configurations may include dependencies distributed under their own licenses. Review the relevant license files for details.
Acknowledgements
Mincord uses and builds upon open-source projects and community documentation, including:
- Discord Userdoccers
- Slint
- Skia
- LiveKit
- WebRTC
- matrix-rust-sdk
- DeepFilterNet
See the repository and dependency license notices for the complete list of third-party components.
<p align="center">
  <sub>Mincord is not affiliated with or endorsed by Discord Inc. Discord is a trademark of Discord Inc.</sub>
</p>
