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
* MQTT pub/sub workers `services/src/mqtt.rs`
* Janus admin interface `services/src/janus.rs`
* System path util `services/src/path.rs`
* Parse Raspberry Pi's `/proc/cpuinfo` fields `services/src/cpuinfo.rs`

## printnanny-cli

```
printnanny
Leigh Johnson <leigh@bitsy.ai>
PrintNanny Command-line Interface

USAGE:
    printnanny-cli [OPTIONS] --config <config> <SUBCOMMAND>

OPTIONS:
    -c, --config <config>    Path to Config.toml (see env/ for examples)
    -h, --help               Print help information
    -v                       Sets the level of verbosity
    -V, --version            Print version information

SUBCOMMANDS:
    device         Interact with device REST API
    factsd         Config serializer (JSON) intended for use with Ansible facts.d
    help           Print this message or the help of the given subcommand(s)
    janus-admin    Interact with Janus admin/monitoring APIs https://janus.conf.meetecho.com/docs/auth.html#token
    monitor        Interact with PrintNanny monitoring service
    mqtt           Interact with MQTT pub/sub service       
```

### Device commands

```
printnanny-cli-device
Leigh Johnson <leigh@bitsy.ai>
Interact with device REST API

USAGE:
    printnanny-cli --config <config> device [OPTIONS] [action]

ARGS:
    <action>    [possible values: get, setup]

OPTIONS:
    -h, --help               Print help information
    -o, --output <output>    
    -V, --version            Print version information   
```

### Ansible facts.d commands

```
printnanny-cli-factsd
Leigh Johnson <leigh@bitsy.ai>
Config serializer (JSON) intended for use with Ansible facts.d

USAGE:
    printnanny-cli --config <config> factsd [OPTIONS]

OPTIONS:
    -h, --help               Print help information
    -o, --output <output>    
    -V, --version            Print version information
```

### Janus Gateway admin commands

```
printnanny-cli-janus-admin
Leigh Johnson <leigh@bitsy.ai>
Interact with Janus admin/monitoring APIs https://janus.conf.meetecho.com/docs/auth.html#token

USAGE:
    printnanny-cli --config <config> janus-admin [OPTIONS] [endpoint]

ARGS:
    <endpoint>    Janus admin/monitoring API endpoint [default: janus.plugin.echotest,janus.plugin.streaming] [possible values: get-status, info,
                  ping, add-token, remove-token, list-tokens, test-stun]

OPTIONS:
        --adminsecret <admin_secret>    [env: JANUS_ADMIN_SECRET]
    -h, --help                          Print help information
    -H, --host <host>                   [default: http://localhost:7088/admin]
        --plugins <plugins>             Commaseparated list of plugins used to scope token access. [default: janus.plugin.echotest,janus.plugin.streaming]
        --token <token>                 [env: JANUS_TOKEN]
    -V, --version                       Print version information
```

### PrintNanny monitoring commands

```
printnanny-cli-monitor
Leigh Johnson <leigh@bitsy.ai>
Interact with PrintNanny monitoring service

USAGE:
    printnanny-cli --config <config> monitor <SUBCOMMAND>

OPTIONS:
    -h, --help       Print help information
    -V, --version    Print version information

SUBCOMMANDS:
    help     Print this message or the help of the given subcommand(s)
    start    Start PrintNanny monitoring service
    stop     Stop PrintNanny monitoring service
```

### MQTT pub/sub commands
```
printnanny-cli-mqtt
Leigh Johnson <leigh@bitsy.ai>
Interact with MQTT pub/sub service

USAGE:
    printnanny-cli --config <config> mqtt --ca-certs <ca_certs> --private-key <private_key> --public-key <public_key> [action]

ARGS:
    <action>    [possible values: publish, subscribe]

OPTIONS:
        --ca-certs <ca_certs>          [env: MQTT_CA_CERTS=]
    -h, --help                         Print help information
        --private-key <private_key>    [env: MQTT_PRIVATE_KEY=]
        --public-key <public_key>      [env: MQTT_PUBLIC_KEY=]
    -V, --version                      Print version information
```

### printnanny-dash

Two-factor authentication dash based on [Rocket.rs](https://rocket.rs/)

### Credits

Created from [XAMPPRocky/mean-bean-ci-template](https://github.com/XAMPPRocky/mean-bean-ci-template)

This is a template for GitHub Actions meant as a successor to [`japaric/trust`](https://github.com/japaric/trust)
for handling [`XAMPPRocky/tokei`](https://github.com/XAMPPRocky/tokei)'s CI and
deployment. I decided to make it a template to be able share it across
projects and with the community.
