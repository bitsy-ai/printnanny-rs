# PrintNanny OS Tools

![Discord](https://img.shields.io/discord/773452324692688956)
![Github Followers](https://img.shields.io/github/followers/leigh-johnson?style=social)

![Commit Activity](https://img.shields.io/github/commit-activity/m/bitsy-ai/printnanny-cli)
![Release](https://img.shields.io/github/release-date-pre/bitsy-ai/printnanny-cli)

![PrintNanny Logo](https://github.com/bitsy-ai/octoprint-nanny-plugin/raw/main/docs/images/logo.jpg)

### [Learn more](https://www.print-nanny.com/)

### [Install PrintNanny OS](https://print-nanny.com/devices/releases/)

# Crates

This workspace contains the following tools used in PrintNanny OS:

## printnanny-services

* Hierarchical config based on [Figment](https://github.com/SergioBenitez/Figment) `services/src/config.rs`
* REST API library `services/src/printnanny_api.rs`
* Janus admin interface `services/src/janus.rs`
* System path util `services/src/path.rs`
* Parse Raspberry Pi's `/proc/cpuinfo` fields `services/src/cpuinfo.rs`

## printnanny-cli

* Documentation @ https://docs.printnanny.ai/docs/command-line-interface/


### printnanny-dash

Two-factor authentication dash based on [Rocket.rs](https://rocket.rs/)

### printnanny-dev

Developer tools

```
Leigh Johnson <leigh@printnanny.ai>
PrintNanny Developer Command-line Tools

USAGE:
    printnanny-dev [OPTIONS] <SUBCOMMAND>

OPTIONS:
    -c, --config <config>    Path to Config.toml (see env/ for examples)
    -h, --help               Print help information
    -v                       Sets the level of verbosity
    -V, --version            Print version information

SUBCOMMANDS:
    ansible      Interact with Ansible installation, vars, and playbooks
    help         Print this message or the help of the given subcommand(s)
    octoprint    Interact with OctoPrint installation
    repetier     Interact with Repetier installation
```

### Credits

Created from [XAMPPRocky/mean-bean-ci-template](https://github.com/XAMPPRocky/mean-bean-ci-template)

This is a template for GitHub Actions meant as a successor to [`japaric/trust`](https://github.com/japaric/trust)
for handling [`XAMPPRocky/tokei`](https://github.com/XAMPPRocky/tokei)'s CI and
deployment. I decided to make it a template to be able share it across
projects and with the community.


